use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Expr, ExprLit, ItemFn, Lit, LitBool, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

struct StringList {
    values: Vec<LitStr>,
}

struct UnstableFeatureArgs {
    name: LitStr,
    is_authorization_required: LitBool,
}

impl Parse for StringList {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut values = Vec::new();

        while !input.is_empty() {
            values.push(input.parse()?);

            if input.is_empty() {
                break;
            }

            input.parse::<Token![,]>()?;
        }

        Ok(Self { values })
    }
}

impl Parse for UnstableFeatureArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![,]>()?;

        let Expr::Lit(ExprLit {
            lit: Lit::Bool(is_authorization_required),
            ..
        }) = input.parse()?
        else {
            return Err(input.error("expected boolean literal"));
        };

        if !input.is_empty() {
            input.parse::<Token![,]>()?;

            if !input.is_empty() {
                return Err(input.error("unexpected extra arguments"));
            }
        }

        Ok(Self {
            name,
            is_authorization_required,
        })
    }
}

#[proc_macro_attribute]
pub fn supported_versions(args: TokenStream, item: TokenStream) -> TokenStream {
    register_string_slice("VERSIONS", "__SUPPORTED_VERSION_", args, item)
}

/// use example
/// #[unstable_feature("org.example.my_feature", true)]
/// async fn router() ...
#[proc_macro_attribute]
pub fn unstable_feature(args: TokenStream, item: TokenStream) -> TokenStream {
    register_unstable_feature(args, item)
}

fn register_string_slice(
    slice_name: &str,
    static_prefix: &str,
    args: TokenStream,
    item: TokenStream,
) -> TokenStream {
    let strings = parse_macro_input!(args as StringList);
    let function = parse_macro_input!(item as ItemFn);

    let function_name = &function.sig.ident;
    let slice_ident = format_ident!("{slice_name}");

    let generated = strings.values.iter().enumerate().map(|(index, value)| {
        let static_ident = format_ident!("{static_prefix}{function_name}_{index}");

        quote! {
            #[::linkme::distributed_slice(crate::state::#slice_ident)]
            static #static_ident: &'static str = #value;
        }
    });

    TokenStream::from(quote! {
        #(#generated)*
        #function
    })
}

fn register_unstable_feature(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as UnstableFeatureArgs);
    let function = parse_macro_input!(item as ItemFn);

    let function_name = &function.sig.ident;
    let static_ident = format_ident!("__UNSTABLE_FEATURE_{function_name}");
    let feature_name = args.name;
    let is_authorization_required = args.is_authorization_required;

    TokenStream::from(quote! {
        #[::linkme::distributed_slice(crate::state::UNSTABLE_FEATURES)]
        static #static_ident: crate::state::UnstableFeature = crate::state::UnstableFeature {
            name: #feature_name,
            is_authorization_required: #is_authorization_required,
        };

        #function
    })
}
