use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::{Map, Value, json};
use syn::{
    Expr, ExprCall, ExprLit, ExprMethodCall, ExprPath, Fields, File, FnArg, GenericArgument,
    Item, ItemEnum, ItemFn, ItemStruct, ItemType, Lit, PatType, PathArguments, ReturnType, Type,
    TypeArray, TypePath, parse_file,
};

fn main() -> Result<(), Box<dyn Error>> {
    let output_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/openapi/openapi.json"));

    let project = ProjectScanner::new()?;
    let document = project.build_openapi_document()?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&output_path, serde_json::to_vec_pretty(&document)?)?;
    println!("{}", output_path.display());
    Ok(())
}

struct ProjectScanner {
    files: HashMap<PathBuf, File>,
    handler_scanners: HashMap<PathBuf, HandlerScanner>,
}

impl ProjectScanner {
    fn new() -> Result<Self, Box<dyn Error>> {
        let mut files = HashMap::new();
        for path in rust_files("src")? {
            let source = fs::read_to_string(&path)?;
            files.insert(path, parse_file(&source)?);
        }

        Ok(Self {
            files,
            handler_scanners: HashMap::new(),
        })
    }

    fn build_openapi_document(mut self) -> Result<OpenApiDocument, Box<dyn Error>> {
        let routes = self.discover_routes()?;
        let mut paths: BTreeMap<String, BTreeMap<String, OpenApiOperation>> = BTreeMap::new();

        for route in routes {
            let scanner = self.handler_scanner(&route.handlers_file)?;
            let operation = scanner.operation_for_route(&route)?;
            paths
                .entry(route.path.clone())
                .or_default()
                .insert(route.method.clone(), operation);
        }

        Ok(OpenApiDocument {
            openapi: "3.1.0",
            info: OpenApiInfo {
                title: "gridstack".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
            },
            paths,
        })
    }

    fn discover_routes(&self) -> Result<Vec<RouteSpec>, Box<dyn Error>> {
        let main_file = PathBuf::from("src/main.rs");
        let root_router = self.find_root_router_file(&main_file)?;
        let root_module_dir = module_dir_for_root_file(&root_router);
        let mut visited = HashSet::new();
        let mut routes = Vec::new();
        self.collect_routes_from_router(&root_router, &root_module_dir, &mut visited, &mut routes)?;
        Ok(routes)
    }

    fn find_root_router_file(&self, main_file: &Path) -> Result<PathBuf, Box<dyn Error>> {
        let file = self
            .files
            .get(main_file)
            .ok_or("src/main.rs not found in file cache")?;

        for item in &file.items {
            let Item::Fn(function) = item else {
                continue;
            };
            if function.sig.ident != "main" {
                continue;
            }

            for statement in &function.block.stmts {
                let syn::Stmt::Local(local) = statement else {
                    continue;
                };

                let Some(init) = &local.init else {
                    continue;
                };

                let Expr::Call(ExprCall { func, .. }) = init.expr.as_ref() else {
                    continue;
                };

                let Expr::Path(ExprPath { path, .. }) = func.as_ref() else {
                    continue;
                };

                let segments: Vec<String> =
                    path.segments.iter().map(|segment| segment.ident.to_string()).collect();
                if segments == ["services".to_owned(), "router".to_owned()] {
                    return Ok(PathBuf::from("src/services.rs"));
                }
            }
        }

        Err("root services::router(...) call not found in main.rs".into())
    }

    fn collect_routes_from_router(
        &self,
        router_file: &Path,
        module_dir: &Path,
        visited: &mut HashSet<PathBuf>,
        routes: &mut Vec<RouteSpec>,
    ) -> Result<(), Box<dyn Error>> {
        if !visited.insert(router_file.to_path_buf()) {
            return Ok(());
        }

        let file = self
            .files
            .get(router_file)
            .ok_or_else(|| format!("router file `{}` not found", router_file.display()))?;

        let router_fn = file
            .items
            .iter()
            .find_map(|item| match item {
                Item::Fn(function) if function.sig.ident == "router" => Some(function),
                _ => None,
            })
            .ok_or_else(|| format!("router function not found in `{}`", router_file.display()))?;

        let expr = router_expression(router_fn)?;
        self.walk_router_expression(expr, module_dir, visited, routes)
    }

    fn walk_router_expression(
        &self,
        expr: &Expr,
        module_dir: &Path,
        visited: &mut HashSet<PathBuf>,
        routes: &mut Vec<RouteSpec>,
    ) -> Result<(), Box<dyn Error>> {
        let Expr::MethodCall(ExprMethodCall {
            receiver,
            method,
            args,
            ..
        }) = expr
        else {
            return Ok(());
        };

        self.walk_router_expression(receiver, module_dir, visited, routes)?;

        match method.to_string().as_str() {
            "route" => {
                let path = route_path(args.first().ok_or("route path missing")?)?;
                let (http_method, handler_name) =
                    route_handler(args.iter().nth(1).ok_or("route handler missing")?)?;
                routes.push(RouteSpec {
                    path,
                    method: http_method,
                    handler_name,
                    handlers_file: module_dir.join("handlers.rs"),
                });
            }
            "merge" => {
                let merged_router = args.first().ok_or("merge target missing")?;
                let (target_router_file, target_module_dir) =
                    resolve_merged_router(module_dir, merged_router)?;
                self.collect_routes_from_router(
                    &target_router_file,
                    &target_module_dir,
                    visited,
                    routes,
                )?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handler_scanner(&mut self, handlers_file: &Path) -> Result<&HandlerScanner, Box<dyn Error>> {
        if !self.handler_scanners.contains_key(handlers_file) {
            let file = self
                .files
                .get(handlers_file)
                .ok_or_else(|| format!("handlers file `{}` not found", handlers_file.display()))?;
            self.handler_scanners
                .insert(handlers_file.to_path_buf(), HandlerScanner::from_file(file.clone()));
        }

        Ok(self
            .handler_scanners
            .get(handlers_file)
            .expect("handler scanner inserted"))
    }
}

struct HandlerScanner {
    structs: HashMap<String, ItemStruct>,
    enums: HashMap<String, ItemEnum>,
    aliases: HashMap<String, ItemType>,
    functions: HashMap<String, ItemFn>,
}

impl HandlerScanner {
    fn from_file(file: File) -> Self {
        let mut structs = HashMap::new();
        let mut enums = HashMap::new();
        let mut aliases = HashMap::new();
        let mut functions = HashMap::new();

        for item in file.items {
            match item {
                Item::Struct(item_struct) => {
                    structs.insert(item_struct.ident.to_string(), item_struct);
                }
                Item::Enum(item_enum) => {
                    enums.insert(item_enum.ident.to_string(), item_enum);
                }
                Item::Type(item_type) => {
                    aliases.insert(item_type.ident.to_string(), item_type);
                }
                Item::Fn(item_fn) => {
                    functions.insert(item_fn.sig.ident.to_string(), item_fn);
                }
                _ => {}
            }
        }

        Self {
            structs,
            enums,
            aliases,
            functions,
        }
    }

    fn operation_for_route(&self, route: &RouteSpec) -> Result<OpenApiOperation, Box<dyn Error>> {
        let function = self
            .functions
            .get(&route.handler_name)
            .ok_or_else(|| format!("handler `{}` not found", route.handler_name))?;

        let mut parameters = Vec::new();
        let mut request_body = None;

        for input in &function.sig.inputs {
            let FnArg::Typed(PatType { ty, .. }) = input else {
                continue;
            };

            let Some((wrapper, inner)) = extract_outer_inner_type(ty.as_ref()) else {
                continue;
            };

            match wrapper.as_str() {
                "Query" => parameters.extend(self.query_parameters(&inner)?),
                "Json" => {
                    request_body = Some(OpenApiRequestBody {
                        required: true,
                        description: format!("Request body for `{}`.", route.handler_name),
                        content: json_content(self.schema_for_type_name(&inner)?),
                    });
                }
                _ => {}
            }
        }

        let (ok_type, err_type) = result_type_names(&function.sig.output)?;
        let success_response = self.response_spec_from_type_name(&ok_type, "200")?;

        let mut responses = BTreeMap::new();
        responses.insert(
            success_response.status,
            OpenApiResponse {
                description: "Successful response.".to_owned(),
                content: Some(json_content(self.schema_for_type_name(
                    success_response
                        .body_type
                        .as_deref()
                        .ok_or("success response body type missing")?,
                )?)),
            },
        );

        for (status, schema, description) in self.error_responses(&err_type)? {
            if let Some(existing) = responses.get_mut(&status) {
                if !existing.description.split(" | ").any(|part| part == description) {
                    existing.description = format!("{} | {}", existing.description, description);
                }
            } else {
                responses.insert(
                    status,
                    OpenApiResponse {
                        description,
                        content: Some(json_content(schema)),
                    },
                );
            }
        }

        Ok(OpenApiOperation {
            operation_id: route.handler_name.clone(),
            summary: None,
            description: None,
            tags: Vec::new(),
            parameters,
            request_body,
            responses,
        })
    }

    fn query_parameters(&self, type_name: &str) -> Result<Vec<OpenApiParameter>, Box<dyn Error>> {
        let item_struct = self
            .structs
            .get(type_name)
            .ok_or_else(|| format!("query struct `{type_name}` not found"))?;

        let Fields::Named(fields) = &item_struct.fields else {
            return Ok(Vec::new());
        };

        let mut parameters = Vec::new();
        for field in &fields.named {
            let field_name =
                serde_field_name(field).unwrap_or_else(|| field.ident.as_ref().unwrap().to_string());
            parameters.push(OpenApiParameter {
                name: field_name,
                location: "query".to_owned(),
                required: !is_option_type(&field.ty) && !has_serde_default(&field.attrs),
                description: String::new(),
                schema: self.schema_for_syn_type(&field.ty)?,
            });
        }

        Ok(parameters)
    }

    fn error_responses(
        &self,
        error_type: &str,
    ) -> Result<Vec<(String, Value, String)>, Box<dyn Error>> {
        let item_enum = self
            .enums
            .get(error_type)
            .ok_or_else(|| format!("error enum `{error_type}` not found"))?;

        let mut responses = Vec::new();

        for variant in &item_enum.variants {
            let response = match &variant.fields {
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    let field_type = &fields.unnamed.first().unwrap().ty;
                    self.response_spec_from_syn_type(field_type, "500")?
                }
                _ => ResponseSpec {
                    status: "500".to_owned(),
                    body_type: None,
                },
            };

            let schema = if let Some(body_type) = response.body_type.as_deref() {
                self.schema_for_type_name(body_type)?
            } else {
                json!({ "type": "object" })
            };

            responses.push((response.status, schema, variant.ident.to_string()));
        }

        responses.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.2.cmp(&right.2)));
        Ok(responses)
    }

    fn response_spec_from_type_name(
        &self,
        type_name: &str,
        default_status: &str,
    ) -> Result<ResponseSpec, Box<dyn Error>> {
        if let Some(alias) = self.aliases.get(type_name) {
            return self.response_spec_from_syn_type(&alias.ty, default_status);
        }

        Ok(ResponseSpec {
            status: default_status.to_owned(),
            body_type: Some(type_name.to_owned()),
        })
    }

    fn response_spec_from_syn_type(
        &self,
        ty: &Type,
        default_status: &str,
    ) -> Result<ResponseSpec, Box<dyn Error>> {
        if let Some(spec) = json_response_spec_from_syn_type(ty) {
            return Ok(spec);
        }

        if let Some(type_name) = simple_type_name(ty) {
            return self.response_spec_from_type_name(&type_name, default_status);
        }

        Err("unsupported response type".into())
    }

    fn schema_for_type_name(&self, type_name: &str) -> Result<Value, Box<dyn Error>> {
        if let Some(alias) = self.aliases.get(type_name) {
            if let Some(spec) = json_response_spec_from_syn_type(&alias.ty)
                && let Some(body_type) = spec.body_type
            {
                return self.schema_for_type_name(&body_type);
            }

            return self.schema_for_syn_type(&alias.ty);
        }

        if let Some(item_struct) = self.structs.get(type_name) {
            return self.schema_for_struct(item_struct);
        }

        if let Some(item_enum) = self.enums.get(type_name) {
            return self.schema_for_enum(item_enum);
        }

        Ok(match type_name {
            "String" => json!({ "type": "string" }),
            "bool" => json!({ "type": "boolean" }),
            "u64" => json!({ "type": "integer", "format": "int64" }),
            "Value" => json!({ "type": "object" }),
            _ => json!({ "type": "object" }),
        })
    }

    fn schema_for_struct(&self, item_struct: &ItemStruct) -> Result<Value, Box<dyn Error>> {
        let Fields::Named(fields) = &item_struct.fields else {
            return Ok(json!({ "type": "object" }));
        };

        let mut properties = Map::new();
        for field in &fields.named {
            let field_name =
                serde_field_name(field).unwrap_or_else(|| field.ident.as_ref().unwrap().to_string());
            properties.insert(field_name, self.schema_for_syn_type(&field.ty)?);
        }

        Ok(Value::Object(Map::from_iter([
            ("type".to_owned(), Value::String("object".to_owned())),
            ("properties".to_owned(), Value::Object(properties)),
        ])))
    }

    fn schema_for_enum(&self, item_enum: &ItemEnum) -> Result<Value, Box<dyn Error>> {
        if item_enum
            .variants
            .iter()
            .all(|variant| matches!(variant.fields, Fields::Unit))
        {
            let values: Vec<String> = item_enum
                .variants
                .iter()
                .map(|variant| {
                    serde_variant_name(variant)
                        .unwrap_or_else(|| variant.ident.to_string().to_lowercase())
                })
                .collect();

            return Ok(json!({ "type": "string", "enum": values }));
        }

        Ok(json!({ "type": "object" }))
    }

    fn schema_for_syn_type(&self, ty: &Type) -> Result<Value, Box<dyn Error>> {
        if let Some(spec) = json_response_spec_from_syn_type(ty)
            && let Some(body_type) = spec.body_type
        {
            return self.schema_for_type_name(&body_type);
        }

        if let Some(inner) = option_inner_type_name(ty) {
            let mut schema = self.schema_for_type_name(&inner)?;
            if let Value::Object(ref mut map) = schema {
                map.insert("nullable".to_owned(), Value::Bool(true));
            }
            return Ok(schema);
        }

        if let Some(inner) = vec_inner_type_name(ty) {
            return Ok(json!({
                "type": "array",
                "items": self.schema_for_type_name(&inner)?,
            }));
        }

        if let Type::Array(TypeArray { elem, .. }) = ty {
            return Ok(json!({
                "type": "array",
                "items": self.schema_for_syn_type(elem)?,
            }));
        }

        if let Some(type_name) = simple_type_name(ty) {
            return self.schema_for_type_name(&type_name);
        }

        Ok(json!({ "type": "object" }))
    }
}

#[derive(Debug, Clone)]
struct RouteSpec {
    path: String,
    method: String,
    handler_name: String,
    handlers_file: PathBuf,
}

#[derive(Debug, Serialize)]
struct OpenApiDocument {
    openapi: &'static str,
    info: OpenApiInfo,
    paths: BTreeMap<String, BTreeMap<String, OpenApiOperation>>,
}

#[derive(Debug, Serialize)]
struct OpenApiInfo {
    title: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct OpenApiOperation {
    operation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parameters: Vec<OpenApiParameter>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "requestBody")]
    request_body: Option<OpenApiRequestBody>,
    responses: BTreeMap<String, OpenApiResponse>,
}

#[derive(Debug, Serialize)]
struct OpenApiParameter {
    name: String,
    #[serde(rename = "in")]
    location: String,
    required: bool,
    description: String,
    schema: Value,
}

#[derive(Debug, Serialize)]
struct OpenApiRequestBody {
    required: bool,
    description: String,
    content: BTreeMap<String, OpenApiMediaType>,
}

#[derive(Debug, Serialize)]
struct OpenApiResponse {
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<BTreeMap<String, OpenApiMediaType>>,
}

#[derive(Debug, Serialize)]
struct OpenApiMediaType {
    schema: Value,
}

#[derive(Debug)]
struct ResponseSpec {
    status: String,
    body_type: Option<String>,
}

fn rust_files(root: impl AsRef<Path>) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    collect_rust_files(root.as_ref(), &mut files)?;
    Ok(files)
}

fn collect_rust_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        collect_rust_files(&entry.path(), files)?;
    }

    Ok(())
}

fn module_dir_for_root_file(file: &Path) -> PathBuf {
    if file.file_name().and_then(|name| name.to_str()) == Some("mod.rs") {
        file.parent().unwrap().to_path_buf()
    } else {
        let stem = file.file_stem().unwrap().to_str().unwrap();
        file.parent().unwrap().join(stem)
    }
}

fn router_expression(function: &ItemFn) -> Result<&Expr, Box<dyn Error>> {
    function
        .block
        .stmts
        .iter()
        .find_map(|stmt| match stmt {
            syn::Stmt::Expr(expr, _) => Some(expr),
            _ => None,
        })
        .ok_or_else(|| "router function expression not found".into())
}

fn route_path(expr: &Expr) -> Result<String, Box<dyn Error>> {
    string_literal_expr(expr).ok_or_else(|| "route path must be a string literal".into())
}

fn route_handler(expr: &Expr) -> Result<(String, String), Box<dyn Error>> {
    let Expr::Call(ExprCall { func, args, .. }) = expr else {
        return Err("route handler must be method call like post(handler)".into());
    };

    let Expr::Path(ExprPath { path, .. }) = func.as_ref() else {
        return Err("route method function path not found".into());
    };

    let http_method = path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
        .ok_or("route method missing")?;

    let handler_expr = args.first().ok_or("route handler missing")?;
    let Expr::Path(ExprPath { path, .. }) = handler_expr else {
        return Err("route handler must be a path".into());
    };

    let handler_name = path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
        .ok_or("handler name missing")?;

    Ok((http_method, handler_name))
}

fn resolve_merged_router(
    module_dir: &Path,
    expr: &Expr,
) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let Expr::Call(ExprCall { func, .. }) = expr else {
        return Err("merge target must be router(...) call".into());
    };

    let Expr::Path(ExprPath { path, .. }) = func.as_ref() else {
        return Err("merge target path not found".into());
    };

    let segments: Vec<String> = path.segments.iter().map(|segment| segment.ident.to_string()).collect();
    if segments.last().map(String::as_str) != Some("router") {
        return Err("merge target must call router".into());
    }

    let module_segments = &segments[..segments.len() - 1];
    let target_module_dir = module_segments
        .iter()
        .fold(module_dir.to_path_buf(), |acc, segment| acc.join(segment));

    Ok((target_module_dir.join("router.rs"), target_module_dir))
}

fn result_type_names(output: &ReturnType) -> Result<(String, String), Box<dyn Error>> {
    let ReturnType::Type(_, ty) = output else {
        return Err("handler must return Result".into());
    };

    let Type::Path(type_path) = ty.as_ref() else {
        return Err("unsupported return type".into());
    };

    let last = type_path.path.segments.last().ok_or("missing return type segment")?;
    if last.ident != "Result" {
        return Err("handler must return Result".into());
    }

    let PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return Err("unsupported Result arguments".into());
    };

    let mut types = arguments.args.iter().filter_map(|argument| match argument {
        GenericArgument::Type(ty) => simple_type_name(ty),
        _ => None,
    });

    let ok_type = types.next().ok_or("missing success type")?;
    let err_type = types.next().ok_or("missing error type")?;
    Ok((ok_type, err_type))
}

fn extract_outer_inner_type(ty: &Type) -> Option<(String, String)> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    let segment = type_path.path.segments.last()?;
    let wrapper = segment.ident.to_string();
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };

    let inner = arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Type(ty) => simple_type_name(ty),
        _ => None,
    })?;

    Some((wrapper, inner))
}

fn simple_type_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        _ => None,
    }
}

fn option_inner_type_name(ty: &Type) -> Option<String> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    let segment = type_path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }

    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };

    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Type(ty) => simple_type_name(ty),
        _ => None,
    })
}

fn vec_inner_type_name(ty: &Type) -> Option<String> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    let segment = type_path.path.segments.last()?;
    if segment.ident != "Vec" {
        return None;
    }

    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };

    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Type(ty) => simple_type_name(ty),
        _ => None,
    })
}

fn is_option_type(ty: &Type) -> bool {
    option_inner_type_name(ty).is_some()
}

fn serde_field_name(field: &syn::Field) -> Option<String> {
    for attribute in &field.attrs {
        if !attribute.path().is_ident("serde") {
            continue;
        }

        let mut rename = None;
        let _ = attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                rename = Some(lit.value());
            }
            Ok(())
        });

        if rename.is_some() {
            return rename;
        }
    }

    None
}

fn serde_variant_name(variant: &syn::Variant) -> Option<String> {
    for attribute in &variant.attrs {
        if !attribute.path().is_ident("serde") {
            continue;
        }

        let mut rename = None;
        let _ = attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                rename = Some(lit.value());
            }
            Ok(())
        });

        if rename.is_some() {
            return rename;
        }
    }

    None
}

fn has_serde_default(attributes: &[syn::Attribute]) -> bool {
    for attribute in attributes {
        if !attribute.path().is_ident("serde") {
            continue;
        }

        let mut has_default = false;
        let _ = attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                has_default = true;
            }
            Ok(())
        });

        if has_default {
            return true;
        }
    }

    false
}

fn json_response_spec_from_syn_type(ty: &Type) -> Option<ResponseSpec> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return None;
    };

    let segment = path.segments.last()?;
    if segment.ident != "JsonResponse" {
        return None;
    }

    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };

    let mut status = None;
    let mut body_type = None;

    for argument in &arguments.args {
        match argument {
            GenericArgument::Const(Expr::Lit(ExprLit { lit: Lit::Int(lit), .. })) => {
                status = Some(lit.base10_digits().to_owned());
            }
            GenericArgument::Type(ty) => {
                body_type = simple_type_name(ty);
            }
            _ => {}
        }
    }

    Some(ResponseSpec {
        status: status?,
        body_type,
    })
}

fn string_literal_expr(expr: &Expr) -> Option<String> {
    let Expr::Lit(ExprLit { lit: Lit::Str(lit), .. }) = expr else {
        return None;
    };

    Some(lit.value())
}

fn json_content(schema: Value) -> BTreeMap<String, OpenApiMediaType> {
    BTreeMap::from([(
        "application/json".to_owned(),
        OpenApiMediaType { schema },
    )])
}
