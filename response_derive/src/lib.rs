use proc_macro::TokenStream;
use quote::quote;
use std::{env, fs, path::PathBuf};
use syn::{
    AngleBracketedGenericArguments, Data, DeriveInput, Expr, ExprAssign, ExprLit, ExprPath, Fields,
    GenericArgument, Lit, Path, PathArguments, Type, TypePath, parse_macro_input,
    punctuated::Punctuated, spanned::Spanned, token::Comma,
};

#[derive(Clone)]
struct ErrorEntry {
    matrix_error: String,
    from: Path,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PayloadKind {
    MatrixErrorResponse,
    MatrixRateLimitErrorResponse,
    Other,
}

fn parse_key(assign: &ExprAssign) -> syn::Result<String> {
    match &*assign.left {
        Expr::Path(path) => Ok(path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default()),
        _ => Err(syn::Error::new(
            assign.left.span(),
            "matrix attribute key must be identifier",
        )),
    }
}

fn parse_error_entry(expr: Expr) -> syn::Result<ErrorEntry> {
    let tuple_expr = match expr {
        Expr::Paren(paren) => *paren.expr,
        other => other,
    };

    let tuple = match tuple_expr {
        Expr::Tuple(tuple) => tuple,
        _ => {
            return Err(syn::Error::new(
                tuple_expr.span(),
                "error entries must be tuple-like: (matrix_error = \"...\", from = Type::Variant)",
            ));
        }
    };
    let tuple_span = tuple.span();

    let mut matrix_error: Option<String> = None;
    let mut from: Option<Path> = None;

    for elem in tuple.elems {
        let assign = match elem {
            Expr::Assign(assign) => assign,
            _ => {
                return Err(syn::Error::new(
                    elem.span(),
                    "error entry fields must be key = value",
                ));
            }
        };

        let key = parse_key(&assign)?;
        if key == "matrix_error" {
            match *assign.right {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(value),
                    ..
                }) => matrix_error = Some(value.value()),
                _ => {
                    return Err(syn::Error::new(
                        assign.right.span(),
                        "matrix_error must be string literal",
                    ));
                }
            }
        } else if key == "from" {
            match *assign.right {
                Expr::Path(ExprPath { path, .. }) => from = Some(path),
                _ => {
                    return Err(syn::Error::new(
                        assign.right.span(),
                        "from must be enum variant path",
                    ));
                }
            }
        } else {
            return Err(syn::Error::new(
                assign.left.span(),
                "error entry supports only matrix_error and from keys",
            ));
        }
    }

    Ok(ErrorEntry {
        matrix_error: matrix_error
            .ok_or_else(|| syn::Error::new(tuple_span, "missing matrix_error in error entry"))?,
        from: from.ok_or_else(|| syn::Error::new(tuple_span, "missing from in error entry"))?,
    })
}

fn extract_error_type(path: &Path) -> Option<Path> {
    let full = quote! { #path }.to_string();
    let (left, _) = full.rsplit_once("::")?;
    let parsed = syn::parse_str::<Path>(left).ok()?;
    if parsed.segments.is_empty() {
        return None;
    }
    Some(parsed)
}

fn is_json_of_named_type(ty: &Type, expected_inner_ident: &str) -> bool {
    let Type::Path(TypePath { path, .. }) = ty else {
        return false;
    };

    let Some(segment) = path.segments.last() else {
        return false;
    };
    if segment.ident != "Json" {
        return false;
    }

    let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
        &segment.arguments
    else {
        return false;
    };

    let Some(GenericArgument::Type(Type::Path(TypePath { path: inner, .. }))) = args.first() else {
        return false;
    };
    inner
        .segments
        .last()
        .map(|s| s.ident == expected_inner_ident)
        .unwrap_or(false)
}

fn payload_kind(ty: &Type) -> PayloadKind {
    if is_json_of_named_type(ty, "MatrixErrorResponse") {
        PayloadKind::MatrixErrorResponse
    } else if is_json_of_named_type(ty, "MatrixRateLimitErrorResponse") {
        PayloadKind::MatrixRateLimitErrorResponse
    } else {
        PayloadKind::Other
    }
}

#[proc_macro_derive(IntoResponseEnum, attributes(matrix))]
pub fn derive_into_response_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident.clone();

    let data_enum = match input.data {
        Data::Enum(data) => data,
        _ => {
            return syn::Error::new(
                input.span(),
                "IntoResponseEnum can only be derived for enums",
            )
            .to_compile_error()
            .into();
        }
    };

    let mut match_arms = Vec::new();
    let mut metadata_entries = Vec::new();
    let mut mapped_error_entries: Vec<(syn::Ident, Vec<ErrorEntry>, PayloadKind)> = Vec::new();
    let mut internal_fallback_variant: Option<syn::Ident> = None;
    let mut ok_inner_type: Option<Type> = None;

    for variant in data_enum.variants {
        let variant_name = variant.ident.clone();

        let payload_ty = match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                fields.unnamed.first().expect("len checked").ty.clone()
            }
            _ => {
                return syn::Error::new(
                    variant.span(),
                    "Each variant must be a single-field tuple variant",
                )
                .to_compile_error()
                .into();
            }
        };

        let mut status_code: Option<u16> = None;
        let mut matrix_error_codes: Vec<String> = Vec::new();
        let mut error_entries_for_variant: Vec<ErrorEntry> = Vec::new();

        for attr in variant.attrs {
            if !attr.path().is_ident("matrix") {
                continue;
            }

            let parser = Punctuated::<Expr, Comma>::parse_terminated;
            let parsed = match attr.parse_args_with(parser) {
                Ok(value) => value,
                Err(error) => return error.to_compile_error().into(),
            };

            for expr in parsed {
                let assign = match expr {
                    Expr::Assign(assign) => assign,
                    _ => {
                        return syn::Error::new(
                            expr.span(),
                            "matrix attribute expects key = value pairs",
                        )
                        .to_compile_error()
                        .into();
                    }
                };

                let key = match parse_key(&assign) {
                    Ok(value) => value,
                    Err(error) => return error.to_compile_error().into(),
                };

                if key == "status" {
                    let lit = match *assign.right {
                        Expr::Lit(ExprLit {
                            lit: Lit::Int(lit), ..
                        }) => lit,
                        _ => {
                            return syn::Error::new(
                                assign.right.span(),
                                "status must be integer literal",
                            )
                            .to_compile_error()
                            .into();
                        }
                    };
                    let code = match lit.base10_parse::<u16>() {
                        Ok(value) => value,
                        Err(error) => {
                            return syn::Error::new(lit.span(), error).to_compile_error().into();
                        }
                    };
                    status_code = Some(code);
                } else if key == "matrix_error" {
                    let arr = match *assign.right {
                        Expr::Array(arr) => arr,
                        _ => {
                            return syn::Error::new(
                                assign.right.span(),
                                "matrix_error must be array of string literals",
                            )
                            .to_compile_error()
                            .into();
                        }
                    };
                    for item in arr.elems {
                        match item {
                            Expr::Lit(ExprLit {
                                lit: Lit::Str(value),
                                ..
                            }) => matrix_error_codes.push(value.value()),
                            _ => {
                                return syn::Error::new(
                                    item.span(),
                                    "matrix_error accepts only string literals",
                                )
                                .to_compile_error()
                                .into();
                            }
                        }
                    }
                } else if key == "error" {
                    let arr = match *assign.right {
                        Expr::Array(arr) => arr,
                        _ => {
                            return syn::Error::new(
                                assign.right.span(),
                                "error must be array of (matrix_error = ..., from = ...)",
                            )
                            .to_compile_error()
                            .into();
                        }
                    };

                    for item in arr.elems {
                        let entry = match parse_error_entry(item) {
                            Ok(value) => value,
                            Err(error) => return error.to_compile_error().into(),
                        };
                        matrix_error_codes.push(entry.matrix_error.clone());
                        error_entries_for_variant.push(entry);
                    }
                } else {
                    return syn::Error::new(
                        assign.left.span(),
                        "matrix attribute supports status, matrix_error, error",
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }

        let current_payload_kind = payload_kind(&payload_ty);

        if !error_entries_for_variant.is_empty() {
            mapped_error_entries.push((
                variant_name.clone(),
                error_entries_for_variant,
                current_payload_kind,
            ));
        }

        if status_code == Some(500) && matrix_error_codes.iter().any(|code| code == "M_UNKNOWN") {
            internal_fallback_variant = Some(variant_name.clone());
        }

        if variant_name == "Ok" {
            if let Type::Path(TypePath { path, .. }) = &payload_ty
                && let Some(segment) = path.segments.last()
                && segment.ident == "Json"
                && let syn::PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    args,
                    ..
                }) = &segment.arguments
                && let Some(GenericArgument::Type(inner)) = args.first()
            {
                ok_inner_type = Some(inner.clone());
            }
        }

        let variant_name_string = variant_name.to_string();
        let status_literal = status_code.unwrap_or(200);
        let single_matrix_error_code = if matrix_error_codes.len() == 1 {
            Some(matrix_error_codes[0].clone())
        } else {
            None
        };
        let matrix_error_literals: Vec<proc_macro2::TokenStream> = matrix_error_codes
            .iter()
            .map(|value| quote! { #value })
            .collect();
        metadata_entries.push(quote! {
            (#variant_name_string, #status_literal, &[#(#matrix_error_literals),*])
        });

        let errcode_patch = if let Some(matrix_code) = single_matrix_error_code {
            if is_json_of_named_type(&payload_ty, "MatrixErrorResponse") {
                quote! {
                    payload.0.errcode = #matrix_code.to_owned();
                    if payload.0.error.is_empty() {
                        payload.0.error = #matrix_code.to_owned();
                    }
                }
            } else if is_json_of_named_type(&payload_ty, "MatrixRateLimitErrorResponse") {
                quote! {
                    payload.0.base.errcode = #matrix_code.to_owned();
                    if payload.0.base.error.is_empty() {
                        payload.0.base.error = #matrix_code.to_owned();
                    }
                }
            } else {
                quote! {}
            }
        } else {
            quote! {}
        };

        let arm = if let Some(code) = status_code {
            quote! {
                Self::#variant_name(mut payload) => {
                    #errcode_patch
                    (
                        ::axum::http::StatusCode::from_u16(#code)
                            .expect("invalid status code in #[matrix(status = ...)]"),
                        payload,
                    )
                        .into_response()
                },
            }
        } else {
            quote! {
                Self::#variant_name(mut payload) => {
                    #errcode_patch
                    payload.into_response()
                },
            }
        };

        match_arms.push(arm);
    }

    let mut error_match_arms = Vec::new();
    let mut error_type: Option<Path> = None;
    for (variant_name, entries, kind) in mapped_error_entries {
        for entry in entries {
            let current_error_type = match extract_error_type(&entry.from) {
                Some(value) => value,
                None => {
                    return syn::Error::new(
                        entry.from.span(),
                        "from path must include error type and variant",
                    )
                    .to_compile_error()
                    .into();
                }
            };

            if let Some(existing) = &error_type {
                if quote! { #existing }.to_string() != quote! { #current_error_type }.to_string() {
                    return syn::Error::new(
                        entry.from.span(),
                        "all mapped errors must come from the same error enum type",
                    )
                    .to_compile_error()
                    .into();
                }
            } else {
                error_type = Some(current_error_type);
            }

            let from_span = entry.from.span();
            let from_path = entry.from;
            let code = entry.matrix_error;
            let arm = match kind {
                PayloadKind::MatrixErrorResponse => quote! {
                    #from_path => Self::#variant_name(::axum::Json(crate::services::shared::MatrixErrorResponse {
                        errcode: #code.to_owned(),
                        error: error.to_string(),
                    })),
                },
                PayloadKind::MatrixRateLimitErrorResponse => quote! {
                    #from_path => Self::#variant_name(::axum::Json(crate::services::shared::MatrixRateLimitErrorResponse {
                        base: crate::services::shared::MatrixErrorResponse {
                            errcode: #code.to_owned(),
                            error: error.to_string(),
                        },
                        retry_after_ms: 0,
                    })),
                },
                PayloadKind::Other => {
                    return syn::Error::new(
                        from_span,
                        "mapped errors require Json<MatrixErrorResponse> or Json<MatrixRateLimitErrorResponse> payload",
                    )
                    .to_compile_error()
                    .into();
                }
            };
            error_match_arms.push(arm);
        }
    }

    let from_error_impl = if let Some(ref err_type) = error_type {
        let fallback = if let Some(variant_name) = internal_fallback_variant {
            quote! {
                Self::#variant_name(::axum::Json(crate::services::shared::MatrixErrorResponse {
                    errcode: "M_UNKNOWN".to_owned(),
                    error: "Internal Error".to_owned(),
                }))
            }
        } else {
            quote! {
                panic!("No #[matrix(status = 500, matrix_error = [\"M_UNKNOWN\"])] variant defined")
            }
        };

        quote! {
            pub fn from_mapped_error(error: #err_type) -> Self {
                match error {
                    #(#error_match_arms)*
                    _ => #fallback,
                }
            }
        }
    } else {
        quote! {}
    };

    let from_result_impl =
        if let (Some(err_type), Some(ok_type)) = (error_type.clone(), ok_inner_type) {
            quote! {
                pub fn from_result(result: ::core::result::Result<#ok_type, #err_type>) -> Self {
                    match result {
                        ::core::result::Result::Ok(value) => Self::Ok(::axum::Json(value)),
                        ::core::result::Result::Err(error) => Self::from_mapped_error(error),
                    }
                }
            }
        } else {
            quote! {}
        };

    let output = quote! {
        impl ::axum::response::IntoResponse for #enum_name {
            fn into_response(self) -> ::axum::response::Response {
                match self {
                    #(#match_arms)*
                }
            }
        }

        impl #enum_name {
            pub const RESPONSE_METADATA: &'static [(&'static str, u16, &'static [&'static str])] =
                &[#(#metadata_entries),*];

            #from_error_impl
            #from_result_impl
        }
    };

    #[cfg(debug_assertions)]
    {
        dump_generated_output_debug_only(&enum_name.to_string(), &output.to_string());
    }
    output.into()
}

fn dump_generated_output_debug_only(enum_name: &str, generated: &str) {
    let output_dir = resolve_output_dir();
    if fs::create_dir_all(&output_dir).is_err() {
        return;
    }

    let sanitized = enum_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let output_path = output_dir.join(format!("{sanitized}.generated.rs"));
    let _ = fs::write(output_path, generated);
}

fn resolve_output_dir() -> PathBuf {
    if let Some(out_dir) = env::var_os("OUT_DIR") {
        return PathBuf::from(out_dir).join("response_derive");
    }
    if let Some(manifest_dir) = env::var_os("CARGO_MANIFEST_DIR") {
        let manifest = PathBuf::from(manifest_dir);

        return manifest.join("target").join("response_derive");
    }
    PathBuf::from("target/response_derive")
}
