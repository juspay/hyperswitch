use proc_macro2::TokenStream;
use quote::quote;

use crate::macros::helpers as macro_helpers;

#[derive(PartialEq)]
enum FieldType {
    String,
    OptionString,
    Other,
}

fn get_field_type(ty: &syn::Type) -> FieldType {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = &segment.ident;
            if ident == "String" {
                return FieldType::String;
            }
            if ident == "Option" {
                if let syn::PathArguments::AngleBracketed(generic_args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(syn::Type::Path(inner_path))) =
                        generic_args.args.first()
                    {
                        if let Some(inner_segment) = inner_path.path.segments.last() {
                            if inner_segment.ident == "String" {
                                return FieldType::OptionString;
                            }
                        }
                    }
                }
            }
        }
    }
    FieldType::Other
}

fn is_xss_clean_skip(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("xss_clean") {
            if let syn::Meta::List(ref meta_list) = attr.meta {
                if let Ok(ident) = meta_list.parse_args::<syn::Ident>() {
                    if ident == "skip" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn validate_xss_or_sqli_derive(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = macro_helpers::get_struct_fields(input.data)
        .map_err(|error| syn::Error::new(proc_macro2::Span::call_site(), error))?;

    let validation_checks = fields
        .iter()
        .filter_map(|field| {
            let field_name = field.ident.as_ref()?;
            if is_xss_clean_skip(field) {
                return None;
            }

            match get_field_type(&field.ty) {
                FieldType::String => Some(quote! {
                    if ::common_utils::validation::contains_potential_xss_or_sqli(&self.#field_name) {
                        return Err(format!(
                            "{} contains potential XSS or SQLi attack vectors",
                            stringify!(#field_name)
                        ));
                    }
                }),
                FieldType::OptionString => Some(quote! {
                    if let Some(ref val) = self.#field_name {
                        if ::common_utils::validation::contains_potential_xss_or_sqli(val) {
                            return Err(format!(
                                "{} contains potential XSS or SQLi attack vectors",
                                stringify!(#field_name)
                            ));
                        }
                    }
                }),
                FieldType::Other => None,
            }
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #name #ty_generics #where_clause {
            pub fn validate_xss_or_sqli(&self) -> Result<(), String> {
                #(#validation_checks)*
                Ok(())
            }
        }
    })
}
