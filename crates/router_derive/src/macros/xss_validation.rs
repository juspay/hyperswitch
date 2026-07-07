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
    let type_str = quote!(#ty).to_string().replace(" ", "");

    if type_str == "String" || type_str.ends_with("::String") {
        FieldType::String
    } else if type_str == "Option<String>" || type_str.ends_with("::Option<String>") {
        FieldType::OptionString
    } else if type_str == "Vec<String>" || type_str.ends_with("::Vec<String>") {
        FieldType::VecString
    } else if type_str == "Option<Vec<String>>" || type_str.ends_with("::Option<Vec<String>>") {
        FieldType::OptionVecString
    } else if type_str == "HashSet<String>" || type_str.ends_with("::HashSet<String>") {
        FieldType::HashSetString
    } else if type_str == "Option<HashSet<String>>"
        || type_str.ends_with("::Option<HashSet<String>>")
    {
        FieldType::OptionHashSetString
    } else {
        FieldType::Other
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
        impl #impl_generics #name #ty_generics #where_clause {
            pub fn validate_xss_or_sqli(&self) -> Result<(), String> {
                Ok(())
                    #(#validation_checks)*
            }
        }
    })
}
