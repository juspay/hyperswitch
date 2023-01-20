mod helpers;

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    punctuated::Punctuated, token::Comma, Data, DeriveInput, Fields, Ident, ImplGenerics,
    TypeGenerics, Variant, WhereClause,
};

use crate::macros::{
    api_error::helpers::{
        check_missing_attributes, get_unused_fields, ErrorTypeProperties, ErrorVariantProperties,
        HasErrorTypeProperties, HasErrorVariantProperties,
    },
    helpers::non_enum_error,
};

pub(crate) fn api_error_derive_inner(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let variants = match &ast.data {
        Data::Enum(e) => &e.variants,
        _ => return Err(non_enum_error()),
    };

    let type_properties = ast.get_type_properties()?;

    let mut variants_properties_map = HashMap::new();
    for variant in variants {
        let variant_properties = variant.get_variant_properties()?;
        check_missing_attributes(variant, &variant_properties)?;

        variants_properties_map.insert(variant, variant_properties);
    }

    let error_type_fn = implement_error_type(name, &type_properties, &variants_properties_map);
    let error_code_fn = implement_error_code(name, &variants_properties_map);
    let error_message_fn = implement_error_message(name, &variants_properties_map);
    let serialize_impl = implement_serialize(
        name,
        (&impl_generics, &ty_generics, where_clause),
        &type_properties,
        &variants_properties_map,
    );

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics std::error::Error for #name #ty_generics #where_clause {}

        #[automatically_derived]
        impl #impl_generics #name #ty_generics #where_clause {
            #error_type_fn
            #error_code_fn
            #error_message_fn
        }

        #serialize_impl
    })
}

fn implement_error_type(
    enum_name: &Ident,
    type_properties: &ErrorTypeProperties,
    variants_properties_map: &HashMap<&Variant, ErrorVariantProperties>,
) -> TokenStream {
    let mut arms = Vec::new();
    for (&variant, properties) in variants_properties_map.iter() {
        let ident = &variant.ident;
        let params = match variant.fields {
            Fields::Unit => quote! {},
            Fields::Unnamed(..) => quote! { (..) },
            Fields::Named(..) => quote! { {..} },
        };
        // Safety: Missing attributes are already checked before this function is called.
        #[allow(clippy::unwrap_used)]
        let error_type = properties.error_type.as_ref().unwrap();

        arms.push(quote! { #enum_name::#ident #params => #error_type });
    }

    // Safety: Missing attributes are already checked before this function is called.
    #[allow(clippy::unwrap_used)]
    let error_type_enum = type_properties.error_type_enum.as_ref().unwrap();
    quote! {
        pub fn error_type(&self) -> #error_type_enum {
            match self {
                #(#arms),*
            }
        }
    }
}

fn implement_error_code(
    enum_name: &Ident,
    variants_properties_map: &HashMap<&Variant, ErrorVariantProperties>,
) -> TokenStream {
    let mut arms = Vec::new();
    for (&variant, properties) in variants_properties_map.iter() {
        let ident = &variant.ident;
        let params = match variant.fields {
            Fields::Unit => quote! {},
            Fields::Unnamed(..) => quote! { (..) },
            Fields::Named(..) => quote! { {..} },
        };
        // Safety: Missing attributes are already checked before this function is called.
        #[allow(clippy::unwrap_used)]
        let error_code = properties.code.as_ref().unwrap();

        arms.push(quote! { #enum_name::#ident #params => #error_code.to_string() });
    }

    quote! {
        pub fn error_code(&self) -> String {
            match self {
                #(#arms),*
            }
        }
    }
}

fn implement_error_message(
    enum_name: &Ident,
    variants_properties_map: &HashMap<&Variant, ErrorVariantProperties>,
) -> TokenStream {
    let mut arms = Vec::new();
    for (&variant, properties) in variants_properties_map.iter() {
        let ident = &variant.ident;
        let params = match variant.fields {
            Fields::Unit => quote! {},
            Fields::Unnamed(..) => quote! { (..) },
            Fields::Named(ref fields) => {
                let fields = fields
                    .named
                    .iter()
                    .map(|f| {
                        // Safety: Named fields are guaranteed to have an identifier.
                        #[allow(clippy::unwrap_used)]
                        f.ident.as_ref().unwrap()
                    })
                    .collect::<Punctuated<&Ident, Comma>>();
                quote! { {#fields} }
            }
        };
        // Safety: Missing attributes are already checked before this function is called.
        #[allow(clippy::unwrap_used)]
        let error_message = properties.message.as_ref().unwrap();

        arms.push(quote! { #enum_name::#ident #params => format!(#error_message) });
    }

    quote! {
        pub fn error_message(&self) -> String {
            match self {
                #(#arms),*
            }
        }
    }
}

fn implement_serialize(
    enum_name: &Ident,
    generics: (&ImplGenerics<'_>, &TypeGenerics<'_>, Option<&WhereClause>),
    type_properties: &ErrorTypeProperties,
    variants_properties_map: &HashMap<&Variant, ErrorVariantProperties>,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics;
    let mut arms = Vec::new();
    for (&variant, properties) in variants_properties_map.iter() {
        let ident = &variant.ident;
        let params = match variant.fields {
            Fields::Unit => quote! {},
            Fields::Unnamed(..) => quote! { (..) },
            Fields::Named(ref fields) => {
                let fields = fields
                    .named
                    .iter()
                    .map(|f| {
                        // Safety: Named fields are guaranteed to have an identifier.
                        #[allow(clippy::unwrap_used)]
                        f.ident.as_ref().unwrap()
                    })
                    .collect::<Punctuated<&Ident, Comma>>();
                quote! { {#fields} }
            }
        };
        // Safety: Missing attributes are already checked before this function is called.
        #[allow(clippy::unwrap_used)]
        let error_message = properties.message.as_ref().unwrap();
        let msg_unused_fields =
            get_unused_fields(&variant.fields, &error_message.value(), &properties.ignore);

        // Safety: Missing attributes are already checked before this function is called.
        #[allow(clippy::unwrap_used)]
        let error_type_enum = type_properties.error_type_enum.as_ref().unwrap();
        let response_definition = if msg_unused_fields.is_empty() {
            quote! {
                #[derive(Clone, Debug, serde::Serialize)]
                struct ErrorResponse {
                    #[serde(rename = "type")]
                    error_type: #error_type_enum,
                    code: String,
                    message: String,
                }
            }
        } else {
            let mut extra_fields = Vec::new();
            for field in &msg_unused_fields {
                let vis = &field.vis;
                // Safety: `msq_unused_fields` is expected to contain named fields only.
                #[allow(clippy::unwrap_used)]
                let ident = &field.ident.as_ref().unwrap();
                let ty = &field.ty;
                extra_fields.push(quote! { #vis #ident: #ty });
            }
            quote! {
                #[derive(Clone, Debug, serde::Serialize)]
                struct ErrorResponse #ty_generics #where_clause {
                    #[serde(rename = "type")]
                    error_type: #error_type_enum,
                    code: String,
                    message: String,
                    #(#extra_fields),*
                }
            }
        };

        // Safety: Missing attributes are already checked before this function is called.
        #[allow(clippy::unwrap_used)]
        let error_type = properties.error_type.as_ref().unwrap();
        // Safety: Missing attributes are already checked before this function is called.
        #[allow(clippy::unwrap_used)]
        let code = properties.code.as_ref().unwrap();
        // Safety: Missing attributes are already checked before this function is called.
        #[allow(clippy::unwrap_used)]
        let message = properties.message.as_ref().unwrap();
        let extra_fields = msg_unused_fields
            .iter()
            .map(|field| {
                // Safety: `extra_fields` is expected to contain named fields only.
                #[allow(clippy::unwrap_used)]
                let field_name = field.ident.as_ref().unwrap();
                quote! { #field_name: #field_name.to_owned() }
            })
            .collect::<Vec<TokenStream>>();
        arms.push(quote! {
            #enum_name::#ident #params => {
                #response_definition
                let response = ErrorResponse {
                    error_type: #error_type,
                    code: #code.to_string(),
                    message: format!(#message),
                    #(#extra_fields),*
                };
                response.serialize(serializer)
            }
        });
    }
    quote! {
        #[automatically_derived]
        impl #impl_generics serde::Serialize for #enum_name #ty_generics #where_clause {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                match self {
                    #(#arms),*
                }
            }
        }
    }
}
