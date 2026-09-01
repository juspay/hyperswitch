use proc_macro2::TokenStream;
use quote::quote;

use crate::macros::helpers as macro_helpers;

#[derive(PartialEq)]
enum FieldType {
    String,
    OptionString,
    VecString,
    OptionVecString,
    HashSetString,
    OptionHashSetString,
    Other,
}

fn get_field_type(ty: &syn::Type) -> FieldType {
    match ty {
        syn::Type::Path(type_path) => {
            type_path
                .path
                .segments
                .last()
                .map_or(FieldType::Other, |segment| {
                    let ident_str = segment.ident.to_string();
                    if ident_str == "String" {
                        FieldType::String
                    } else if ident_str == "Option" {
                        match &segment.arguments {
                            syn::PathArguments::AngleBracketed(args) => {
                                args.args.first().map_or(FieldType::Other, |arg| match arg {
                                    syn::GenericArgument::Type(inner_type) => {
                                        match get_field_type(inner_type) {
                                            FieldType::String => FieldType::OptionString,
                                            FieldType::VecString => FieldType::OptionVecString,
                                            FieldType::HashSetString => {
                                                FieldType::OptionHashSetString
                                            }
                                            _ => FieldType::Other,
                                        }
                                    }
                                    _ => FieldType::Other,
                                })
                            }
                            _ => FieldType::Other,
                        }
                    } else if ident_str == "Vec" {
                        match &segment.arguments {
                            syn::PathArguments::AngleBracketed(args) => {
                                args.args.first().map_or(FieldType::Other, |arg| match arg {
                                    syn::GenericArgument::Type(inner_type) => {
                                        match get_field_type(inner_type) {
                                            FieldType::String => FieldType::VecString,
                                            _ => FieldType::Other,
                                        }
                                    }
                                    _ => FieldType::Other,
                                })
                            }
                            _ => FieldType::Other,
                        }
                    } else if ident_str == "HashSet" {
                        match &segment.arguments {
                            syn::PathArguments::AngleBracketed(args) => {
                                args.args.first().map_or(FieldType::Other, |arg| match arg {
                                    syn::GenericArgument::Type(inner_type) => {
                                        match get_field_type(inner_type) {
                                            FieldType::String => FieldType::HashSetString,
                                            _ => FieldType::Other,
                                        }
                                    }
                                    _ => FieldType::Other,
                                })
                            }
                            _ => FieldType::Other,
                        }
                    } else {
                        FieldType::Other
                    }
                })
        }
        _ => FieldType::Other,
    }
}

fn is_xss_clean_skip(field: &syn::Field) -> bool {
    field.attrs.iter().any(|attr| {
        if attr.path().is_ident("xss_clean") {
            if let syn::Meta::List(ref meta_list) = attr.meta {
                if let Ok(ident) = meta_list.parse_args::<syn::Ident>() {
                    ident == "skip"
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    })
}

fn is_xss_clean_recurse(field: &syn::Field) -> bool {
    field.attrs.iter().any(|attr| {
        if attr.path().is_ident("xss_clean") {
            if let syn::Meta::List(ref meta_list) = attr.meta {
                if let Ok(ident) = meta_list.parse_args::<syn::Ident>() {
                    ident == "recurse"
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    })
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
                None
            } else if is_xss_clean_recurse(field) {
                Some(quote! {
                    .and_then(|_| {
                        ::common_utils::validation::ValidateXSSOrSQLi::validate_xss_or_sqli(&self.#field_name)
                    })
                })
            } else {
                match get_field_type(&field.ty) {
                    FieldType::String => Some(quote! {
                        .and_then(|_| {
                            if ::common_utils::validation::contains_potential_xss_or_sqli(&self.#field_name) {
                                Err(format!(
                                    "{} contains potential XSS or SQLi attack vectors",
                                    stringify!(#field_name)
                                ))
                            } else {
                                Ok(())
                            }
                        })
                    }),
                    FieldType::OptionString => Some(quote! {
                        .and_then(|_| {
                            if let Some(ref val) = self.#field_name {
                                if ::common_utils::validation::contains_potential_xss_or_sqli(val) {
                                    Err(format!(
                                        "{} contains potential XSS or SQLi attack vectors",
                                        stringify!(#field_name)
                                    ))
                                } else {
                                    Ok(())
                                }
                            } else {
                                Ok(())
                            }
                        })
                    }),
                    FieldType::VecString => Some(quote! {
                        .and_then(|_| {
                            self.#field_name.iter().try_for_each(|val| {
                                if ::common_utils::validation::contains_potential_xss_or_sqli(val) {
                                    Err(format!(
                                        "{} element contains potential XSS or SQLi attack vectors",
                                        stringify!(#field_name)
                                    ))
                                } else {
                                    Ok(())
                                }
                            })
                        })
                    }),
                    FieldType::OptionVecString => Some(quote! {
                        .and_then(|_| {
                            if let Some(ref vec) = self.#field_name {
                                vec.iter().try_for_each(|val| {
                                    if ::common_utils::validation::contains_potential_xss_or_sqli(val) {
                                        Err(format!(
                                            "{} element contains potential XSS or SQLi attack vectors",
                                            stringify!(#field_name)
                                        ))
                                    } else {
                                        Ok(())
                                    }
                                })
                            } else {
                                Ok(())
                            }
                        })
                    }),
                    FieldType::HashSetString => Some(quote! {
                        .and_then(|_| {
                            self.#field_name.iter().try_for_each(|val| {
                                if ::common_utils::validation::contains_potential_xss_or_sqli(val) {
                                    Err(format!(
                                        "{} element contains potential XSS or SQLi attack vectors",
                                        stringify!(#field_name)
                                    ))
                                } else {
                                    Ok(())
                                }
                            })
                        })
                    }),
                    FieldType::OptionHashSetString => Some(quote! {
                        .and_then(|_| {
                            if let Some(ref set) = self.#field_name {
                                set.iter().try_for_each(|val| {
                                    if ::common_utils::validation::contains_potential_xss_or_sqli(val) {
                                        Err(format!(
                                            "{} element contains potential XSS or SQLi attack vectors",
                                            stringify!(#field_name)
                                        ))
                                    } else {
                                        Ok(())
                                    }
                                })
                            } else {
                                Ok(())
                            }
                        })
                    }),
                    FieldType::Other => None,
                }
            }
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics common_utils::validation::ValidateXSSOrSQLi for #name #ty_generics #where_clause {
            fn validate_xss_or_sqli(&self) -> Result<(), String> {
                Ok(())
                    #(#validation_checks)*
            }
        }
    })
}
