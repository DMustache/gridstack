use std::{
    collections::{BTreeMap, HashMap},
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::{Map, Value, json};
use syn::{
    Expr, File, FnArg, GenericArgument, Item, LitInt, LitStr, Meta, PatType, PathArguments,
    ReturnType, Type,
};

fn main() -> Result<()> {
    let output = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("openapi.json"));

    let routes_file = Path::new("src/services/authorization/routes.rs");
    let handlers_dir = Path::new("src/services/authorization/handlers");
    let handlers_file = handlers_dir.join("../handlers.rs");
    let shared_file = Path::new("src/services/shared.rs");

    let operations = extract_operations(routes_file)?;
    let handler_specs = extract_handler_specs(&handlers_file)?;
    let response_specs = extract_response_specs(&handlers_file)?;

    let mut schemas = extract_schemas_from_dir(handlers_dir)?;
    schemas.extend(extract_schemas_from_file(shared_file)?);

    let mut paths: BTreeMap<String, Value> = BTreeMap::new();
    for op in operations {
        let method_object = build_method_object(&op, &handler_specs, &response_specs, &schemas)?;

        let entry = paths
            .entry(op.path)
            .or_insert_with(|| Value::Object(Map::new()));
        let object = entry
            .as_object_mut()
            .context("path entry must be an object")?;
        object.insert(op.method, method_object);
    }

    let mut schema_map = Map::new();
    for (name, schema) in schemas {
        schema_map.insert(name, schema);
    }

    let openapi_value = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Gridstack Authorization API",
            "version": "0.1.0",
            "description": "Auto-generated from src/services/authorization routes and handlers."
        },
        "paths": paths,
        "components": {
            "schemas": schema_map
        }
    });

    let openapi: utoipa::openapi::OpenApi =
        serde_json::from_value(openapi_value).context("invalid generated OpenAPI structure")?;
    let content = serde_json::to_string_pretty(&openapi)?;

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, content)?;
    println!("OpenAPI written to {}", output.display());
    Ok(())
}

#[derive(Debug)]
struct RouteOperation {
    path: String,
    method: String,
    operation_id: String,
}

#[derive(Debug, Clone)]
struct HandlerSpec {
    query_type: Option<String>,
    json_body_type: Option<String>,
    response_enum: Option<String>,
}

#[derive(Debug, Clone)]
struct ResponseVariantSpec {
    status: String,
    schema_type: Option<String>,
    matrix_error_codes: Vec<String>,
}

fn extract_operations(routes_file: &Path) -> Result<Vec<RouteOperation>> {
    let content = fs::read_to_string(routes_file)
        .with_context(|| format!("failed to read {}", routes_file.display()))?;
    let parsed: File = syn::parse_file(&content)
        .with_context(|| format!("failed to parse {}", routes_file.display()))?;

    let mut operations = Vec::new();
    for item in parsed.items {
        if let Item::Fn(function) = item {
            collect_route_calls_from_block(&function.block.stmts, &mut operations);
        }
    }

    if operations.is_empty() {
        bail!("no authorization routes were detected in {}", routes_file.display());
    }

    Ok(operations)
}

fn collect_route_calls_from_block(stmts: &[syn::Stmt], operations: &mut Vec<RouteOperation>) {
    for stmt in stmts {
        if let syn::Stmt::Expr(expr, _) = stmt {
            collect_route_calls_from_expr(expr, operations);
        }
    }
}

fn collect_route_calls_from_expr(expr: &Expr, operations: &mut Vec<RouteOperation>) {
    if let Expr::MethodCall(call) = expr {
        collect_route_calls_from_expr(&call.receiver, operations);

        if call.method == "route"
            && call.args.len() == 2
            && let Some(Expr::Lit(expr_lit)) = call.args.first()
            && let syn::Lit::Str(path_lit) = &expr_lit.lit
            && let Some(route_definition) = call.args.iter().nth(1)
        {
            let path = path_lit.value();
            collect_http_methods(route_definition, &path, operations);
        }
    }
}

fn collect_http_methods(expr: &Expr, path: &str, operations: &mut Vec<RouteOperation>) {
    if let Expr::Call(call) = expr
        && let Expr::Path(path_expr) = &*call.func
        && let Some(segment) = path_expr.path.segments.last()
    {
        let method = segment.ident.to_string();
        if matches!(method.as_str(), "get" | "post" | "put" | "patch" | "delete")
            && let Some(Expr::Path(handler_path)) = call.args.first()
            && let Some(handler_segment) = handler_path.path.segments.last()
        {
            operations.push(RouteOperation {
                path: path.to_owned(),
                method,
                operation_id: handler_segment.ident.to_string(),
            });
        }
        return;
    }

    if let Expr::MethodCall(call) = expr {
        collect_http_methods(&call.receiver, path, operations);
        let method = call.method.to_string();
        if matches!(method.as_str(), "get" | "post" | "put" | "patch" | "delete")
            && let Some(Expr::Path(handler_path)) = call.args.first()
            && let Some(handler_segment) = handler_path.path.segments.last()
        {
            operations.push(RouteOperation {
                path: path.to_owned(),
                method,
                operation_id: handler_segment.ident.to_string(),
            });
        }
    }
}

fn extract_handler_specs(handlers_file: &Path) -> Result<HashMap<String, HandlerSpec>> {
    let content = fs::read_to_string(handlers_file)
        .with_context(|| format!("failed to read {}", handlers_file.display()))?;
    let parsed: File = syn::parse_file(&content)
        .with_context(|| format!("failed to parse {}", handlers_file.display()))?;

    let mut specs = HashMap::new();
    for item in parsed.items {
        let Item::Fn(function) = item else {
            continue;
        };

        let handler_name = function.sig.ident.to_string();
        let mut query_type = None;
        let mut json_body_type = None;

        for input in &function.sig.inputs {
            let FnArg::Typed(PatType { ty, .. }) = input else {
                continue;
            };

            if let Some(inner) = extract_inner_type_name(ty, "Query") {
                query_type = Some(inner);
            }
            if let Some(inner) = extract_inner_type_name(ty, "Json") {
                json_body_type = Some(inner);
            }
        }

        let response_enum = match &function.sig.output {
            ReturnType::Type(_, ty) => type_name_from_type(ty),
            ReturnType::Default => None,
        };

        specs.insert(
            handler_name,
            HandlerSpec {
                query_type,
                json_body_type,
                response_enum,
            },
        );
    }

    Ok(specs)
}

fn extract_response_specs(handlers_file: &Path) -> Result<HashMap<String, Vec<ResponseVariantSpec>>> {
    let content = fs::read_to_string(handlers_file)
        .with_context(|| format!("failed to read {}", handlers_file.display()))?;
    let parsed: File = syn::parse_file(&content)
        .with_context(|| format!("failed to parse {}", handlers_file.display()))?;

    let mut specs = HashMap::new();
    for item in parsed.items {
        let Item::Enum(item_enum) = item else {
            continue;
        };

        if !has_into_response_enum_derive(&item_enum.attrs) {
            continue;
        }

        let enum_name = item_enum.ident.to_string();
        let mut variants = Vec::new();

        for variant in item_enum.variants {
            let mut status = None;
            let mut matrix_error_codes = Vec::new();
            for attr in &variant.attrs {
                if !attr.path().is_ident("matrix") {
                    continue;
                }

                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("status") {
                        let lit: LitInt = meta.value()?.parse()?;
                        status = Some(lit.base10_digits().to_owned());
                    }
                    Ok(())
                });

                let token_str = match &attr.meta {
                    Meta::List(list) => list.tokens.to_string(),
                    _ => String::new(),
                };
                matrix_error_codes.extend(extract_matrix_error_codes_from_tokens(&token_str)?);
            }

            let payload_type = variant
                .fields
                .iter()
                .next()
                .and_then(|field| extract_inner_type_name(&field.ty, "Json"));

            variants.push(ResponseVariantSpec {
                status: status.unwrap_or_else(|| "200".to_owned()),
                schema_type: payload_type,
                matrix_error_codes,
            });
        }

        specs.insert(enum_name, variants);
    }

    Ok(specs)
}

fn has_into_response_enum_derive(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("derive") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("IntoResponseEnum") {
                found = true;
            }
            Ok(())
        });
        found
    })
}

fn extract_matrix_error_codes_from_tokens(tokens: &str) -> Result<Vec<String>> {
    let string_code_re = Regex::new(r#"matrix_error\s*=\s*\"([^\"]+)\""#)?;
    let list_code_re = Regex::new(r#"matrix_error\s*=\s*\[([^\]]+)\]"#)?;
    let quoted_re = Regex::new(r#"\"([^\"]+)\""#)?;

    let mut codes = Vec::new();
    for cap in string_code_re.captures_iter(tokens) {
        codes.push(cap[1].to_owned());
    }
    for cap in list_code_re.captures_iter(tokens) {
        let inner = cap[1].to_owned();
        for quoted in quoted_re.captures_iter(&inner) {
            codes.push(quoted[1].to_owned());
        }
    }

    codes.sort();
    codes.dedup();
    Ok(codes)
}

fn build_method_object(
    op: &RouteOperation,
    handler_specs: &HashMap<String, HandlerSpec>,
    response_specs: &HashMap<String, Vec<ResponseVariantSpec>>,
    schemas: &HashMap<String, Value>,
) -> Result<Value> {
    let handler = handler_specs.get(&op.operation_id);

    let mut method = Map::new();
    method.insert("tags".to_owned(), json!(["authorization"]));
    method.insert("operationId".to_owned(), json!(op.operation_id));

    if let Some(query_type) = handler.and_then(|h| h.query_type.as_ref()) {
        let parameters = build_query_parameters(query_type, schemas)?;
        if !parameters.is_empty() {
            method.insert("parameters".to_owned(), Value::Array(parameters));
        }
    }

    if let Some(body_type) = handler.and_then(|h| h.json_body_type.as_ref()) {
        method.insert(
            "requestBody".to_owned(),
            json!({
                "required": true,
                "content": {
                    "application/json": {
                        "schema": { "$ref": format!("#/components/schemas/{}", body_type) }
                    }
                }
            }),
        );
    }

    let responses = if let Some(response_enum) = handler.and_then(|h| h.response_enum.as_ref()) {
        build_responses(response_specs.get(response_enum))
    } else {
        build_responses(None)
    };
    method.insert("responses".to_owned(), responses);

    Ok(Value::Object(method))
}

fn build_query_parameters(query_type: &str, schemas: &HashMap<String, Value>) -> Result<Vec<Value>> {
    let Some(schema) = schemas.get(query_type) else {
        return Ok(Vec::new());
    };

    let object = schema
        .as_object()
        .with_context(|| format!("schema {} must be object", query_type))?;
    let props = object
        .get("properties")
        .and_then(Value::as_object)
        .with_context(|| format!("schema {} missing properties", query_type))?;

    let required: Vec<String> = object
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();

    let mut parameters = Vec::new();
    for (name, schema) in props {
        parameters.push(json!({
            "name": name,
            "in": "query",
            "required": required.iter().any(|r| r == name),
            "schema": schema,
        }));
    }

    Ok(parameters)
}

fn build_responses(specs: Option<&Vec<ResponseVariantSpec>>) -> Value {
    let mut responses = Map::new();

    if let Some(specs) = specs {
        for spec in specs {
            let mut response = Map::new();
            response.insert("description".to_owned(), json!("Response"));

            if let Some(schema_type) = &spec.schema_type {
                response.insert(
                    "content".to_owned(),
                    json!({
                        "application/json": {
                            "schema": { "$ref": format!("#/components/schemas/{}", schema_type) }
                        }
                    }),
                );
            }

            if !spec.matrix_error_codes.is_empty() {
                response.insert(
                    "x-matrix-error-codes".to_owned(),
                    json!(spec.matrix_error_codes),
                );
            }

            responses.insert(spec.status.clone(), Value::Object(response));
        }
    }

    if responses.is_empty() {
        responses.insert("200".to_owned(), json!({ "description": "Success" }));
    }

    Value::Object(responses)
}

fn extract_schemas_from_dir(dir: &Path) -> Result<HashMap<String, Value>> {
    let mut schemas = HashMap::new();
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("rs") {
            continue;
        }
        schemas.extend(extract_schemas_from_file(&path)?);
    }
    Ok(schemas)
}

fn extract_schemas_from_file(path: &Path) -> Result<HashMap<String, Value>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let parsed: File =
        syn::parse_file(&content).with_context(|| format!("failed to parse {}", path.display()))?;

    let mut schemas = HashMap::new();
    for item in parsed.items {
        if let Item::Struct(item_struct) = item {
            let struct_name = item_struct.ident.to_string();
            let fields = match item_struct.fields {
                syn::Fields::Named(named) => named.named,
                _ => continue,
            };

            let mut properties = Map::new();
            let mut required = Vec::new();

            for field in fields {
                let field_name = field.ident.context("missing field name")?.to_string();
                let (serialized_name, optional, schema) =
                    convert_field(&field_name, &field.ty, &field.attrs);
                if !optional {
                    required.push(serialized_name.clone());
                }
                properties.insert(serialized_name, schema);
            }

            let mut schema = Map::new();
            schema.insert("type".to_owned(), json!("object"));
            schema.insert("properties".to_owned(), Value::Object(properties));
            if !required.is_empty() {
                schema.insert("required".to_owned(), json!(required));
            }

            schemas.insert(struct_name, Value::Object(schema));
        }
    }

    Ok(schemas)
}

fn convert_field(field_name: &str, field_type: &Type, attrs: &[syn::Attribute]) -> (String, bool, Value) {
    let (serialized_name, optional_from_attr) = parse_serde_attrs(field_name, attrs);
    let (schema, optional_from_type) = type_to_schema(field_type);
    (serialized_name, optional_from_attr || optional_from_type, schema)
}

fn parse_serde_attrs(field_name: &str, attrs: &[syn::Attribute]) -> (String, bool) {
    let mut name = field_name.to_owned();
    let mut optional = false;

    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value: LitStr = meta.value()?.parse()?;
                name = value.value();
            }
            if meta.path.is_ident("skip_serializing_if") {
                optional = true;
            }
            Ok(())
        });
    }

    (name, optional)
}

fn type_to_schema(ty: &Type) -> (Value, bool) {
    match ty {
        Type::Path(path) => {
            let segment = match path.path.segments.last() {
                Some(segment) => segment,
                None => {
                    return (
                        json!({ "type": "object", "additionalProperties": true }),
                        false,
                    );
                }
            };

            let ident = segment.ident.to_string();
            if ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments
                    && let Some(GenericArgument::Type(inner)) = args.args.first()
                {
                    let (inner_schema, _) = type_to_schema(inner);
                    return (inner_schema, true);
                }
            }

            if ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments
                    && let Some(GenericArgument::Type(inner)) = args.args.first()
                {
                    let (inner_schema, _) = type_to_schema(inner);
                    return (json!({ "type": "array", "items": inner_schema }), false);
                }
            }

            match ident.as_str() {
                "String" => (json!({ "type": "string" }), false),
                "bool" => (json!({ "type": "boolean" }), false),
                "i64" | "i32" | "u64" | "u32" | "usize" => {
                    (json!({ "type": "integer" }), false)
                }
                "Value" => (json!({ "type": "object", "additionalProperties": true }), false),
                _ => (
                    json!({ "$ref": format!("#/components/schemas/{}", ident) }),
                    false,
                ),
            }
        }
        _ => (json!({ "type": "object", "additionalProperties": true }), false),
    }
}

fn extract_inner_type_name(ty: &Type, wrapper: &str) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };

    let last = path.path.segments.last()?;
    if last.ident != wrapper {
        return None;
    }

    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };

    let inner = args.args.first()?;
    let GenericArgument::Type(inner_type) = inner else {
        return None;
    };

    type_name_from_type(inner_type)
}

fn type_name_from_type(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    path.path.segments.last().map(|s| s.ident.to_string())
}
