#[allow(clippy::all)]
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use syn::{
    Expr, ExprAssign, ExprLit, ExprPath, File, FnArg, GenericArgument, Item, Lit, LitInt, LitStr,
    Meta, PatType, PathArguments, ReturnType, Type, UseTree, parse::Parser, punctuated::Punctuated,
    token::Comma,
};

fn main() -> Result<()> {
    let output = env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("openapi.json"), PathBuf::from);

    let services_root = Path::new("src/services");
    let route_files = discover_route_files(services_root)?;
    let source_files = discover_source_files(services_root)?;

    if route_files.is_empty() {
        bail!(
            "no service route files were detected in {}",
            services_root.display()
        );
    }

    let entities_dir = Path::new("src/services/authorization/entities");
    let layer_responses_file = Path::new("src/services/errors.rs");
    let shared_file = Path::new("src/services/shared.rs");

    let operations = extract_operations(&route_files)?;
    let handler_specs = extract_handler_specs_from_files(&source_files)?;
    let response_specs = extract_response_specs_from_files(&source_files)?;
    let layer_response_specs = extract_response_specs(layer_responses_file)?;
    let source_paths = source_files
        .iter()
        .map(PathBuf::as_path)
        .collect::<Vec<_>>();
    let error_messages = collect_error_messages(&source_paths)?;
    let response_specs = enrich_response_specs(response_specs, &error_messages);
    let layer_response_specs = enrich_response_specs(layer_response_specs, &error_messages);

    let mut schemas = extract_schemas_from_files(&source_files)?;
    schemas.extend(extract_schemas_from_dir(entities_dir)?);
    schemas.extend(extract_schemas_from_file(shared_file)?);
    let schema_aliases = extract_schema_aliases_from_files(&source_files)?;

    let normalized_schemas = schemas
        .iter()
        .map(|(name, schema)| {
            (
                name.clone(),
                normalize_schema_refs(schema, &schema_aliases, &schemas),
            )
        })
        .collect::<HashMap<_, _>>();

    let mut paths: BTreeMap<String, Value> = BTreeMap::new();
    for op in operations {
        let method_object = build_method_object(
            &op,
            &handler_specs,
            &response_specs,
            &layer_response_specs,
            &normalized_schemas,
            &schema_aliases,
        )?;

        let entry = paths
            .entry(op.path)
            .or_insert_with(|| Value::Object(Map::new()));
        let object = entry
            .as_object_mut()
            .context("path entry must be an object")?;
        object.insert(op.method, method_object);
    }

    let referenced_schemas = retain_referenced_schemas(&paths, &normalized_schemas);

    let mut schema_map = Map::new();
    for (name, schema) in referenced_schemas {
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
            "schemas": schema_map,
            "securitySchemes": {
                "accessTokenBearer": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "opaque"
                },
                "accessTokenQuery": {
                    "type": "apiKey",
                    "in": "query",
                    "name": "access_token"
                }
            }
        },
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
    service_tag: String,
    layers: Vec<String>,
}

#[derive(Debug, Clone)]
struct HandlerSpec {
    query_type: Option<String>,
    json_body_schema: Option<Value>,
    response_enum: Option<String>,
    auth_requirement: AuthRequirement,
}

#[derive(Debug, Clone)]
struct ResponseVariantSpec {
    status: String,
    schema: Option<Value>,
    schema_type_name: Option<String>,
    matrix_error_codes: Vec<String>,
    matrix_error_messages: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthRequirement {
    None,
    Required,
    Optional,
}

fn discover_route_files(services_root: &Path) -> Result<Vec<PathBuf>> {
    let mut route_files = Vec::new();
    for path in discover_source_files(services_root)? {
        if path.file_name().and_then(|v| v.to_str()) == Some("routes.rs") {
            route_files.push(path);
        }
    }
    route_files.sort();
    Ok(route_files)
}

fn discover_source_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in
            fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, files)?;
            } else if path.extension().and_then(|v| v.to_str()) == Some("rs") {
                files.push(path);
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    walk(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn service_tag_from_routes_file(routes_file: &Path) -> String {
    routes_file
        .parent()
        .and_then(Path::file_name)
        .and_then(|v| v.to_str())
        .map_or_else(|| "services".to_owned(), ToOwned::to_owned)
}

fn extract_operations(route_files: &[PathBuf]) -> Result<Vec<RouteOperation>> {
    let mut operations = Vec::new();
    for routes_file in route_files {
        let content = fs::read_to_string(routes_file)
            .with_context(|| format!("failed to read {}", routes_file.display()))?;
        let parsed: File = syn::parse_file(&content)
            .with_context(|| format!("failed to parse {}", routes_file.display()))?;
        let service_tag = service_tag_from_routes_file(routes_file);

        for item in parsed.items {
            if let Item::Fn(function) = item {
                collect_route_calls_from_block(
                    &function.block.stmts,
                    &service_tag,
                    &mut operations,
                );
            }
        }
    }

    if operations.is_empty() {
        bail!("no service routes were detected");
    }

    Ok(operations)
}

fn collect_route_calls_from_block(
    stmts: &[syn::Stmt],
    service_tag: &str,
    operations: &mut Vec<RouteOperation>,
) {
    for stmt in stmts {
        if let syn::Stmt::Expr(expr, _) = stmt {
            let mut steps = Vec::new();
            flatten_router_steps(expr, &mut steps);
            apply_router_steps(&steps, service_tag, operations);
        }
    }
}

enum RouterStep {
    Route { path: String, route_expr: Expr },
    Layer { layer_name: String },
}

fn flatten_router_steps(expr: &Expr, steps: &mut Vec<RouterStep>) {
    if let Expr::MethodCall(call) = expr {
        flatten_router_steps(&call.receiver, steps);

        if call.method == "route"
            && call.args.len() == 2
            && let Some(Expr::Lit(expr_lit)) = call.args.first()
            && let syn::Lit::Str(path_lit) = &expr_lit.lit
            && let Some(route_definition) = call.args.iter().nth(1)
        {
            steps.push(RouterStep::Route {
                path: path_lit.value(),
                route_expr: route_definition.clone(),
            });
        }

        if call.method == "layer"
            && let Some(layer_name) = call.args.first().and_then(extract_layer_name)
        {
            steps.push(RouterStep::Layer { layer_name });
        }
    }
}

fn extract_layer_name(expr: &Expr) -> Option<String> {
    let Expr::Call(call) = expr else {
        return None;
    };
    let Expr::Path(path_expr) = &*call.func else {
        return None;
    };
    let last = path_expr.path.segments.last()?;
    if last.ident != "from_extractor_with_state" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    let first = args.args.first()?;
    let GenericArgument::Type(Type::Path(layer_type)) = first else {
        return None;
    };
    layer_type.path.segments.last().map(|s| s.ident.to_string())
}

fn apply_router_steps(
    steps: &[RouterStep],
    service_tag: &str,
    operations: &mut Vec<RouteOperation>,
) {
    for step in steps {
        match step {
            RouterStep::Route { path, route_expr } => {
                collect_http_methods(route_expr, path, service_tag, operations);
            }
            RouterStep::Layer { layer_name } => {
                for op in operations.iter_mut() {
                    if !op.layers.iter().any(|layer| layer == layer_name) {
                        op.layers.push(layer_name.clone());
                    }
                }
            }
        }
    }
}

fn collect_http_methods(
    expr: &Expr,
    path: &str,
    service_tag: &str,
    operations: &mut Vec<RouteOperation>,
) {
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
                service_tag: service_tag.to_owned(),
                layers: Vec::new(),
            });
        }
        return;
    }

    if let Expr::MethodCall(call) = expr {
        collect_http_methods(&call.receiver, path, service_tag, operations);
        let method = call.method.to_string();
        if matches!(method.as_str(), "get" | "post" | "put" | "patch" | "delete")
            && let Some(Expr::Path(handler_path)) = call.args.first()
            && let Some(handler_segment) = handler_path.path.segments.last()
        {
            operations.push(RouteOperation {
                path: path.to_owned(),
                method,
                operation_id: handler_segment.ident.to_string(),
                service_tag: service_tag.to_owned(),
                layers: Vec::new(),
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
        let mut json_body_schema = None;
        let mut auth_requirement = AuthRequirement::None;

        for input in &function.sig.inputs {
            let FnArg::Typed(PatType { ty, .. }) = input else {
                continue;
            };

            if let Some(inner) = extract_inner_type_name(ty, "Query") {
                query_type = Some(inner);
            }
            if let Some(schema) = extract_inner_type_schema(ty, "Json") {
                json_body_schema = Some(schema);
            }
            auth_requirement =
                merge_auth_requirement(auth_requirement, extract_auth_requirement(ty));
        }

        let response_enum = match &function.sig.output {
            ReturnType::Type(_, ty) => type_name_from_type(ty),
            ReturnType::Default => None,
        };

        specs.insert(
            handler_name,
            HandlerSpec {
                query_type,
                json_body_schema,
                response_enum,
                auth_requirement,
            },
        );
    }

    Ok(specs)
}

fn extract_handler_specs_from_files(paths: &[PathBuf]) -> Result<HashMap<String, HandlerSpec>> {
    let mut all_specs = HashMap::new();
    for path in paths {
        let specs = extract_handler_specs(path)?;
        all_specs.extend(specs);
    }
    Ok(all_specs)
}

const fn merge_auth_requirement(
    current: AuthRequirement,
    incoming: AuthRequirement,
) -> AuthRequirement {
    match (current, incoming) {
        (AuthRequirement::Required, _) | (_, AuthRequirement::Required) => {
            AuthRequirement::Required
        }
        (AuthRequirement::Optional, _) | (_, AuthRequirement::Optional) => {
            AuthRequirement::Optional
        }
        _ => AuthRequirement::None,
    }
}

fn extract_auth_requirement(ty: &Type) -> AuthRequirement {
    let Some(inner) = extract_inner_type_name(ty, "Extension") else {
        return AuthRequirement::None;
    };
    if inner == "UserId" {
        return AuthRequirement::Required;
    }
    if inner == "Option" {
        // Handle Extension<Option<UserId>>
        if let Type::Path(path) = ty
            && let Some(extension_seg) = path.path.segments.last()
            && extension_seg.ident == "Extension"
            && let PathArguments::AngleBracketed(extension_args) = &extension_seg.arguments
            && let Some(GenericArgument::Type(Type::Path(opt_path))) = extension_args.args.first()
            && let Some(opt_seg) = opt_path.path.segments.last()
            && opt_seg.ident == "Option"
            && let PathArguments::AngleBracketed(opt_args) = &opt_seg.arguments
            && let Some(GenericArgument::Type(Type::Path(user_path))) = opt_args.args.first()
            && user_path
                .path
                .segments
                .last()
                .is_some_and(|path_segment| path_segment.ident == "UserId")
        {
            return AuthRequirement::Optional;
        }
    }
    AuthRequirement::None
}

fn extract_response_specs(
    handlers_file: &Path,
) -> Result<HashMap<String, Vec<ResponseVariantSpec>>> {
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
            let mut matrix_error_sources = HashMap::new();
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

                if let Meta::List(list) = &attr.meta {
                    let parser = Punctuated::<Expr, Comma>::parse_terminated;
                    if let Ok(args) = parser.parse2(list.tokens.clone()) {
                        for expr in args {
                            let Expr::Assign(ExprAssign { left, right, .. }) = expr else {
                                continue;
                            };
                            let key = match *left {
                                Expr::Path(path) => path
                                    .path
                                    .segments
                                    .last()
                                    .map(|v| v.ident.to_string())
                                    .unwrap_or_default(),
                                _ => continue,
                            };

                            if key == "matrix_error" {
                                if let Expr::Array(arr) = *right {
                                    for item in arr.elems {
                                        if let Expr::Lit(ExprLit {
                                            lit: Lit::Str(s), ..
                                        }) = item
                                        {
                                            matrix_error_codes.push(s.value());
                                        }
                                    }
                                }
                            } else if key == "error"
                                && let Expr::Array(arr) = *right
                            {
                                for item in arr.elems {
                                    if let Some((code, from_path)) = parse_error_entry(item) {
                                        matrix_error_codes.push(code.clone());
                                        matrix_error_sources.insert(code, from_path);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let payload_type_name = variant
                .fields
                .iter()
                .next()
                .and_then(|field| extract_inner_type_name(&field.ty, "Json"));
            let payload_schema = variant
                .fields
                .iter()
                .next()
                .and_then(|field| extract_inner_type_schema(&field.ty, "Json"));

            variants.push(ResponseVariantSpec {
                status: status.unwrap_or_else(|| "200".to_owned()),
                schema: payload_schema,
                schema_type_name: payload_type_name,
                matrix_error_codes,
                matrix_error_messages: matrix_error_sources,
            });
        }

        specs.insert(enum_name, variants);
    }

    Ok(specs)
}

fn extract_response_specs_from_files(
    paths: &[PathBuf],
) -> Result<HashMap<String, Vec<ResponseVariantSpec>>> {
    let mut all_specs = HashMap::new();
    for path in paths {
        let specs = extract_response_specs(path)?;
        all_specs.extend(specs);
    }
    Ok(all_specs)
}

fn parse_error_entry(expr: Expr) -> Option<(String, String)> {
    let tuple = match expr {
        Expr::Paren(p) => match *p.expr {
            Expr::Tuple(t) => t,
            _ => return None,
        },
        Expr::Tuple(t) => t,
        _ => return None,
    };

    let mut code = None;
    let mut from = None;
    for elem in tuple.elems {
        let Expr::Assign(ExprAssign { left, right, .. }) = elem else {
            continue;
        };
        let key = match *left {
            Expr::Path(path) => path
                .path
                .segments
                .last()
                .map(|v| v.ident.to_string())
                .unwrap_or_default(),
            _ => continue,
        };
        if key == "matrix_error" {
            if let Expr::Lit(ExprLit {
                lit: Lit::Str(s), ..
            }) = *right
            {
                code = Some(s.value());
            }
        } else if key == "from"
            && let Expr::Path(ExprPath { path, .. }) = *right
        {
            from = Some(path_to_enum_variant_key(&path));
        }
    }
    Some((code?, from?))
}

fn path_to_enum_variant_key(path: &syn::Path) -> String {
    let segments = path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>();
    if segments.len() >= 2 {
        format!(
            "{}::{}",
            segments[segments.len() - 2],
            segments[segments.len() - 1]
        )
    } else {
        segments.first().cloned().unwrap_or_default()
    }
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

fn collect_error_messages(paths: &[&Path]) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for path in paths {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let parsed: File = syn::parse_file(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;

        for item in parsed.items {
            let Item::Enum(item_enum) = item else {
                continue;
            };
            let has_error_derive = item_enum.attrs.iter().any(|attr| {
                if !attr.path().is_ident("derive") {
                    return false;
                }
                let mut found = false;
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("Error") {
                        found = true;
                    }
                    Ok(())
                });
                found
            });
            if !has_error_derive {
                continue;
            }

            let enum_name = item_enum.ident.to_string();
            for variant in item_enum.variants {
                let variant_name = variant.ident.to_string();
                for attr in variant.attrs {
                    if !attr.path().is_ident("error") {
                        continue;
                    }
                    if let Ok(message) = attr.parse_args::<LitStr>() {
                        map.insert(format!("{enum_name}::{variant_name}"), message.value());
                    }
                }
            }
        }
    }
    Ok(map)
}

fn enrich_response_specs(
    mut specs: HashMap<String, Vec<ResponseVariantSpec>>,
    error_messages: &HashMap<String, String>,
) -> HashMap<String, Vec<ResponseVariantSpec>> {
    for variants in specs.values_mut() {
        for variant in variants {
            let sources = variant.matrix_error_messages.clone();
            let mut mapped = HashMap::new();
            for (code, source) in sources {
                if let Some(msg) = error_messages.get(&source) {
                    mapped.insert(code, msg.clone());
                }
            }
            variant.matrix_error_messages = mapped;
        }
    }
    specs
}

fn build_method_object(
    op: &RouteOperation,
    handler_specs: &HashMap<String, HandlerSpec>,
    response_specs: &HashMap<String, Vec<ResponseVariantSpec>>,
    layer_response_specs: &HashMap<String, Vec<ResponseVariantSpec>>,
    schemas: &HashMap<String, Value>,
    schema_aliases: &HashMap<String, String>,
) -> Result<Value> {
    let handler = handler_specs.get(&op.operation_id);

    let mut method = Map::new();
    method.insert("tags".to_owned(), json!([op.service_tag]));
    method.insert("operationId".to_owned(), json!(op.operation_id));

    let mut parameters = build_path_parameters(&op.path);
    if let Some(query_type) = handler.and_then(|h| h.query_type.as_ref()) {
        let resolved_query_type = resolve_schema_alias(query_type, schema_aliases, schemas);
        parameters.extend(build_query_parameters(&resolved_query_type, schemas)?);
    }
    if !parameters.is_empty() {
        method.insert("parameters".to_owned(), Value::Array(parameters));
    }

    if let Some(body_schema) = handler.and_then(|h| h.json_body_schema.as_ref()) {
        let normalized_body_schema = normalize_schema_refs(body_schema, schema_aliases, schemas);
        method.insert(
            "requestBody".to_owned(),
            json!({
                "required": true,
                "content": {
                    "application/json": {
                        "schema": normalized_body_schema
                    }
                }
            }),
        );
    }

    let handler_responses = handler
        .and_then(|h| h.response_enum.as_ref())
        .and_then(|name| response_specs.get(name));
    let mut layer_variants = Vec::new();
    for layer_name in &op.layers {
        if let Some(layer_enum_name) = layer_response_enum_name(layer_name)
            && let Some(spec) = layer_response_specs.get(layer_enum_name)
        {
            layer_variants.push(spec.clone());
        }
    }
    let responses = build_responses(handler_responses, &layer_variants, schema_aliases, schemas);
    method.insert("responses".to_owned(), responses);
    let auth_requirement = handler.map_or(AuthRequirement::None, |handler_specification| {
        handler_specification.auth_requirement
    });
    match auth_requirement {
        AuthRequirement::Required => {
            method.insert(
                "security".to_owned(),
                json!([
                    {"accessTokenQuery": []},
                    {"accessTokenBearer": []}
                ]),
            );
        }
        AuthRequirement::Optional => {
            method.insert(
                "security".to_owned(),
                json!([
                    {},
                    {"accessTokenQuery": []},
                    {"accessTokenBearer": []}
                ]),
            );
            method.insert(
                "description".to_owned(),
                json!(
                    "Optional authorization: endpoint can be called with or without access token."
                ),
            );
        }
        AuthRequirement::None => {}
    }

    Ok(Value::Object(method))
}

fn layer_response_enum_name(layer_name: &str) -> Option<&'static str> {
    match layer_name {
        "AuthorizationLayer" => Some("AuthorizationLayerResponse"),
        "RateLimitLayer" => Some("RateLimitLayerResponse"),
        _ => None,
    }
}

fn build_query_parameters(
    query_type: &str,
    schemas: &HashMap<String, Value>,
) -> Result<Vec<Value>> {
    let Some(schema) = schemas.get(query_type) else {
        return Ok(Vec::new());
    };

    let object = schema
        .as_object()
        .with_context(|| format!("schema {query_type} must be object"))?;
    let props = object
        .get("properties")
        .and_then(Value::as_object)
        .with_context(|| format!("schema {query_type} missing properties"))?;

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

fn build_path_parameters(path: &str) -> Vec<Value> {
    let mut parameters = Vec::new();
    let mut seen = BTreeSet::new();
    let mut remaining = path;

    while let Some(start_index) = remaining.find('{') {
        let tail = &remaining[start_index + 1..];
        let Some(end_index) = tail.find('}') else {
            break;
        };

        let parameter_name = tail[..end_index].trim();
        if !parameter_name.is_empty() && seen.insert(parameter_name.to_owned()) {
            parameters.push(json!({
                "name": parameter_name,
                "in": "path",
                "required": true,
                "schema": { "type": "string" }
            }));
        }

        remaining = &tail[end_index + 1..];
    }

    parameters
}

fn build_responses(
    handler_specs: Option<&Vec<ResponseVariantSpec>>,
    layer_specs: &[Vec<ResponseVariantSpec>],
    schema_aliases: &HashMap<String, String>,
    schemas: &HashMap<String, Value>,
) -> Value {
    let mut merged_by_status: BTreeMap<String, ResponseVariantSpec> = BTreeMap::new();

    if let Some(specs) = handler_specs {
        for spec in specs {
            merge_response_spec(&mut merged_by_status, spec.clone());
        }
    }
    for specs in layer_specs {
        for spec in specs {
            merge_response_spec(&mut merged_by_status, spec.clone());
        }
    }

    let mut responses = Map::new();
    for (status, spec) in merged_by_status {
        let mut response = Map::new();
        let description = build_response_description(&spec);
        response.insert("description".to_owned(), json!(description));

        if let Some(schema) = &spec.schema {
            let mut media = Map::new();
            media.insert(
                "schema".to_owned(),
                normalize_schema_refs(schema, schema_aliases, schemas),
            );
            if let Some(schema_type_name) = &spec.schema_type_name
                && let Some(example) = build_response_example(&spec, schema_type_name)
            {
                media.insert("example".to_owned(), example);
            }
            response.insert(
                "content".to_owned(),
                json!({
                    "application/json": {
                        "schema": media.get("schema").cloned().unwrap_or_else(|| json!({})),
                        "example": media.get("example").cloned().unwrap_or( Value::Null)
                    }
                }),
            );
            if let Some(content) = response.get_mut("content").and_then(Value::as_object_mut)
                && let Some(app_json) = content
                    .get_mut("application/json")
                    .and_then(Value::as_object_mut)
                && app_json.get("example") == Some(&Value::Null)
            {
                app_json.remove("example");
            }
        }

        if !spec.matrix_error_codes.is_empty() {
            response.insert(
                "x-matrix-error-codes".to_owned(),
                json!(spec.matrix_error_codes),
            );
        }

        responses.insert(status, Value::Object(response));
    }

    if responses.is_empty() {
        responses.insert("200".to_owned(), json!({ "description": "Success" }));
    }
    Value::Object(responses)
}

fn build_response_example(spec: &ResponseVariantSpec, schema_type: &str) -> Option<Value> {
    let code = spec.matrix_error_codes.first()?;
    let message = spec
        .matrix_error_messages
        .get(code)
        .cloned()
        .unwrap_or_else(|| "Unknown error".to_owned());

    if schema_type == "MatrixRateLimitErrorResponse" {
        return Some(json!({
            "errcode": code,
            "error": message,
            "retry_after_ms": 0
        }));
    }
    if schema_type == "MatrixErrorResponse" {
        return Some(json!({
            "errcode": code,
            "error": message
        }));
    }
    None
}

fn build_response_description(spec: &ResponseVariantSpec) -> String {
    if spec.matrix_error_codes.is_empty() {
        return "Response".to_owned();
    }
    let listed = spec.matrix_error_codes.join(", ");
    let first = &spec.matrix_error_codes[0];
    let msg = spec
        .matrix_error_messages
        .get(first)
        .map_or("An unknown error occurred", String::as_str);
    format!("Possible errcodes: {listed}. Example: {first} - {msg}")
}

fn merge_response_spec(
    merged_by_status: &mut BTreeMap<String, ResponseVariantSpec>,
    mut incoming: ResponseVariantSpec,
) {
    let entry = merged_by_status
        .entry(incoming.status.clone())
        .or_insert_with(|| ResponseVariantSpec {
            status: incoming.status.clone(),
            schema: incoming.schema.clone(),
            schema_type_name: incoming.schema_type_name.clone(),
            matrix_error_codes: Vec::new(),
            matrix_error_messages: HashMap::new(),
        });

    if entry.schema.is_none() {
        entry.schema = incoming.schema.take();
    }
    if entry.schema_type_name.is_none() {
        entry.schema_type_name = incoming.schema_type_name.take();
    }
    entry
        .matrix_error_codes
        .append(&mut incoming.matrix_error_codes);
    let mut seen = std::collections::HashSet::new();
    entry
        .matrix_error_codes
        .retain(|code| seen.insert(code.clone()));
    for (code, msg) in incoming.matrix_error_messages {
        entry.matrix_error_messages.entry(code).or_insert(msg);
    }
}

fn retain_referenced_schemas(
    paths: &BTreeMap<String, Value>,
    schemas: &HashMap<String, Value>,
) -> BTreeMap<String, Value> {
    let mut required = BTreeSet::new();
    for path_item in paths.values() {
        collect_schema_refs_from_value(path_item, &mut required);
    }

    let mut queue = required.iter().cloned().collect::<Vec<_>>();
    while let Some(name) = queue.pop() {
        let Some(schema) = schemas.get(&name) else {
            continue;
        };

        let mut nested = BTreeSet::new();
        collect_schema_refs_from_value(schema, &mut nested);
        for nested_name in nested {
            if required.insert(nested_name.clone()) {
                queue.push(nested_name);
            }
        }
    }

    let mut filtered = BTreeMap::new();
    for name in required {
        if let Some(schema) = schemas.get(&name) {
            filtered.insert(name, schema.clone());
        }
    }
    filtered
}

fn collect_schema_refs_from_value(value: &Value, refs: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get("$ref") {
                const COMPONENT_PREFIX: &str = "#/components/schemas/";
                if let Some(name) = reference.strip_prefix(COMPONENT_PREFIX) {
                    refs.insert(name.to_owned());
                }
            }
            for child in object.values() {
                collect_schema_refs_from_value(child, refs);
            }
        }
        Value::Array(array) => {
            for child in array {
                collect_schema_refs_from_value(child, refs);
            }
        }
        _ => {}
    }
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

fn extract_schemas_from_files(paths: &[PathBuf]) -> Result<HashMap<String, Value>> {
    let mut schemas = HashMap::new();
    for path in paths {
        schemas.extend(extract_schemas_from_file(path)?);
    }
    Ok(schemas)
}

fn extract_schemas_from_file(path: &Path) -> Result<HashMap<String, Value>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let parsed: File =
        syn::parse_file(&content).with_context(|| format!("failed to parse {}", path.display()))?;

    let mut schemas = HashMap::new();
    for item in parsed.items {
        match item {
            Item::Struct(item_struct) => {
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
            Item::Enum(item_enum) => {
                let enum_name = item_enum.ident.to_string();
                let schema = enum_to_schema(&item_enum);
                schemas.insert(enum_name, schema);
            }
            _ => {}
        }
    }

    Ok(schemas)
}

fn extract_schema_aliases_from_files(paths: &[PathBuf]) -> Result<HashMap<String, String>> {
    let mut aliases = HashMap::new();
    for path in paths {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let parsed: File = syn::parse_file(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;

        for item in parsed.items {
            match item {
                Item::Use(item_use) => {
                    collect_schema_aliases_from_use_tree(&item_use.tree, &mut aliases);
                }
                Item::Type(item_type) => {
                    if let Some(target) = type_name_from_type(&item_type.ty) {
                        aliases.insert(item_type.ident.to_string(), target);
                    }
                }
                _ => {}
            }
        }
    }
    Ok(aliases)
}

fn collect_schema_aliases_from_use_tree(use_tree: &UseTree, aliases: &mut HashMap<String, String>) {
    match use_tree {
        UseTree::Path(path) => collect_schema_aliases_from_use_tree(&path.tree, aliases),
        UseTree::Rename(rename) => {
            aliases.insert(rename.rename.to_string(), rename.ident.to_string());
        }
        UseTree::Group(group) => {
            for nested in &group.items {
                collect_schema_aliases_from_use_tree(nested, aliases);
            }
        }
        UseTree::Name(_) | UseTree::Glob(_) => {}
    }
}

fn resolve_schema_alias(
    schema_name: &str,
    aliases: &HashMap<String, String>,
    schemas: &HashMap<String, Value>,
) -> String {
    let mut current = schema_name.to_owned();
    let mut visited = std::collections::HashSet::new();

    while let Some(next) = aliases.get(&current) {
        if !visited.insert(current.clone()) {
            break;
        }
        current.clone_from(next);
    }

    if schemas.contains_key(&current) {
        current
    } else if schemas.contains_key(schema_name) {
        schema_name.to_owned()
    } else {
        current
    }
}

fn normalize_schema_refs(
    value: &Value,
    aliases: &HashMap<String, String>,
    schemas: &HashMap<String, Value>,
) -> Value {
    match value {
        Value::Object(object) => {
            let mut normalized = Map::new();
            for (key, child) in object {
                if key == "$ref"
                    && let Value::String(reference) = child
                    && let Some(name) = reference.strip_prefix("#/components/schemas/")
                {
                    let resolved = resolve_schema_alias(name, aliases, schemas);
                    normalized.insert(
                        key.clone(),
                        Value::String(format!("#/components/schemas/{resolved}")),
                    );
                } else {
                    normalized.insert(key.clone(), normalize_schema_refs(child, aliases, schemas));
                }
            }
            Value::Object(normalized)
        }
        Value::Array(array) => Value::Array(
            array
                .iter()
                .map(|child| normalize_schema_refs(child, aliases, schemas))
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn enum_to_schema(item_enum: &syn::ItemEnum) -> Value {
    let rename_all = serde_rename_all(&item_enum.attrs);
    let mut variants = Vec::new();
    let mut has_payload = false;

    for variant in &item_enum.variants {
        if variant_serde_other(&variant.attrs) {
            continue;
        }

        if matches!(&variant.fields, syn::Fields::Unit) {
            variants.push(enum_variant_serialized_name(
                &variant.ident.to_string(),
                &variant.attrs,
                rename_all.as_deref(),
            ));
        } else {
            has_payload = true;
            break;
        }
    }

    if !has_payload && !variants.is_empty() {
        return json!({
            "type": "string",
            "enum": variants,
        });
    }

    json!({
        "type": "object",
        "additionalProperties": true
    })
}

fn serde_rename_all(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }

        let mut rename_all = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                let value: LitStr = meta.value()?.parse()?;
                rename_all = Some(value.value());
            }
            Ok(())
        });
        if rename_all.is_some() {
            return rename_all;
        }
    }
    None
}

fn variant_serde_other(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }

        let mut is_other = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("other") {
                is_other = true;
            }
            Ok(())
        });
        if is_other {
            return true;
        }
    }
    false
}

fn enum_variant_serialized_name(
    variant_name: &str,
    attrs: &[syn::Attribute],
    rename_all: Option<&str>,
) -> String {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }

        let mut renamed = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value: LitStr = meta.value()?.parse()?;
                renamed = Some(value.value());
            }
            Ok(())
        });
        if let Some(name) = renamed {
            return name;
        }
    }

    apply_rename_rule(variant_name, rename_all)
}

fn apply_rename_rule(name: &str, rename_all: Option<&str>) -> String {
    match rename_all {
        Some("snake_case") => {
            let mut out = String::new();
            for (index, ch) in name.chars().enumerate() {
                if ch.is_uppercase() {
                    if index > 0 {
                        out.push('_');
                    }
                    for lower in ch.to_lowercase() {
                        out.push(lower);
                    }
                } else {
                    out.push(ch);
                }
            }
            out
        }
        _ => name.to_owned(),
    }
}

fn convert_field(
    field_name: &str,
    field_type: &Type,
    attrs: &[syn::Attribute],
) -> (String, bool, Value) {
    let (serialized_name, optional_from_attr) = parse_serde_attrs(field_name, attrs);
    let (schema, optional_from_type) = type_to_schema(field_type);
    (
        serialized_name,
        optional_from_attr || optional_from_type,
        schema,
    )
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
            let Some(segment) = path.path.segments.last() else {
                return (
                    json!({ "type": "object", "additionalProperties": true }),
                    false,
                );
            };

            let ident = segment.ident.to_string();
            if ident == "Option"
                && let PathArguments::AngleBracketed(args) = &segment.arguments
                && let Some(GenericArgument::Type(inner)) = args.args.first()
            {
                let (inner_schema, _) = type_to_schema(inner);
                return (inner_schema, true);
            }

            if ident == "Vec"
                && let PathArguments::AngleBracketed(args) = &segment.arguments
                && let Some(GenericArgument::Type(inner)) = args.args.first()
            {
                let (inner_schema, _) = type_to_schema(inner);
                return (json!({ "type": "array", "items": inner_schema }), false);
            }

            if matches!(ident.as_str(), "Arc" | "Box")
                && let PathArguments::AngleBracketed(args) = &segment.arguments
                && let Some(GenericArgument::Type(inner)) = args.args.first()
            {
                return type_to_schema(inner);
            }

            if ident == "BTreeMap" || ident == "HashMap" || ident == "Map" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    let mut iter = args.args.iter();
                    let _key = iter.next();
                    if let Some(GenericArgument::Type(value_ty)) = iter.next() {
                        let (value_schema, _) = type_to_schema(value_ty);
                        return (
                            json!({
                                "type": "object",
                                "additionalProperties": value_schema
                            }),
                            false,
                        );
                    }
                }
                return (
                    json!({
                        "type": "object",
                        "additionalProperties": true
                    }),
                    false,
                );
            }

            match ident.as_str() {
                "String" => (json!({ "type": "string" }), false),
                "bool" => (json!({ "type": "boolean" }), false),
                "i64" | "i32" | "u64" | "u32" | "usize" => (json!({ "type": "integer" }), false),
                "Value" => (
                    json!({ "type": "object", "additionalProperties": true }),
                    false,
                ),
                _ => (
                    json!({ "$ref": format!("#/components/schemas/{}", ident) }),
                    false,
                ),
            }
        }
        _ => (
            json!({ "type": "object", "additionalProperties": true }),
            false,
        ),
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

fn extract_inner_type_schema(ty: &Type, wrapper: &str) -> Option<Value> {
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

    let (schema, _) = type_to_schema(inner_type);
    Some(schema)
}

fn type_name_from_type(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    path.path.segments.last().map(|s| s.ident.to_string())
}
