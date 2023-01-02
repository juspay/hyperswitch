use std::fmt;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, DeriveInput};

#[derive(Debug)]
enum MacroError {
    SynError(syn::Error),
    NotFound { message: String, span: Span },
    FormatError { message: String, span: Span },
}

impl std::error::Error for MacroError {}
impl fmt::Display for MacroError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl From<syn::Error> for MacroError {
    fn from(value: syn::Error) -> Self {
        Self::SynError(value)
    }
}

pub fn auto_into_derive_inner(token: proc_macro::TokenStream) -> proc_macro::TokenStream {
    error_unwrap(auto_into_derive(token))
}

fn auto_into_derive(token: proc_macro::TokenStream) -> Result<TokenStream, MacroError> {
    let input = syn::parse::<DeriveInput>(token)?;
    let ident = &input.ident;
    let attributes = get_supported_structs(&input.attrs).ok_or(MacroError::NotFound {
        message: "no attributes for `#[converts(...)]` found".to_string(),
        span: ident.span(),
    })?;

    let struct_list = convert_to_ident(attributes)?;

    let fields: Vec<syn::Ident> = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = input.data
    {
        Ok(named)
    } else {
        Err(MacroError::NotFound {
            message: "unable to find fields in the current struct".to_string(),
            span: Span::call_site(),
        })
    }?
    .into_iter()
    .filter_map(|f| f.ident.clone())
    .collect();

    let tokens = struct_list.into_iter().map(|struct_name| {
        conversion(
            ident.clone().into_token_stream(),
            struct_name,
            fields.clone(),
        )
    });
    let output = quote! {
        #(#tokens)*
    };

    Ok(output)
}

fn get_supported_structs(attr: &[syn::Attribute]) -> Option<&syn::Attribute> {
    attr.iter().find(|attr_inner| {
        attr_inner
            .path
            .get_ident()
            .map(|ident| *ident == "converts")
            .unwrap_or(false)
    })
}

fn convert_to_ident(attr: &syn::Attribute) -> Result<Vec<TokenStream>, MacroError> {
    let meta = attr.parse_meta()?;
    match meta {
        syn::Meta::List(syn::MetaList { nested, .. }) => {
            Ok(nested.into_iter().filter_map(get_ident).collect())
        }
        _ => Err(MacroError::FormatError {
            message: "unable to detect a list of similar structs to be parsed".to_string(),
            span: attr.span(),
        }),
    }
}

fn get_ident(item: syn::NestedMeta) -> Option<TokenStream> {
    match item {
        syn::NestedMeta::Meta(inner) => Some(inner.to_token_stream()),
        _ => None,
    }
}

fn conversion(
    struct_a: TokenStream,
    struct_b: TokenStream,
    fields: Vec<syn::Ident>,
) -> TokenStream {
    let from_a_to_b = {
        let fields = fields.iter();
        quote! {
            impl From<#struct_a> for #struct_b {
                fn from(item: #struct_a) -> #struct_b {
                    #struct_b {
                        #(#fields: item.#fields.into(),)*
                    }
                }
            }
        }
    };

    let from_b_to_a = {
        let fields = fields.iter();
        quote! {
            impl From<#struct_b> for #struct_a {
                fn from(item: #struct_b) -> #struct_a {
                    #struct_a {
                        #(#fields: item.#fields.into(),)*
                    }
                }
            }
        }
    };

    quote! {
        #from_a_to_b

        #from_b_to_a
    }
}

impl MacroError {
    fn to_compile_error(&self) -> TokenStream {
        match self {
            Self::SynError(err) => err.to_compile_error(),
            Self::NotFound { message, span } => syn::Error::new(*span, message).to_compile_error(),
            Self::FormatError { message, span } => {
                syn::Error::new(*span, message).to_compile_error()
            }
        }
    }
}

fn error_unwrap(res: Result<TokenStream, MacroError>) -> proc_macro::TokenStream {
    match res {
        Ok(value) => value.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
