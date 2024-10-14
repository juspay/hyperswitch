use crate::macros::{helpers::get_struct_fields, misc::get_field_type};
use quote::{format_ident, quote};
use std::iter::Iterator;
use syn::{punctuated::Punctuated, token::Comma, Field};

#[derive(PartialEq)]
enum Type {
    Encrypted,
    Decrypted,
    DecryptedUpdate,
    FromRequest,
    Updated,
}

fn get_inner_type(path: &syn::TypePath) -> syn::Result<syn::TypePath> {
    if let Some(segment) = path.path.segments.last() {
        if let syn::PathArguments::AngleBracketed(ref args) = &segment.arguments {
            if let Some(syn::GenericArgument::Type(ty)) = args.args.first() {
                if let syn::Type::Path(path) = ty.clone() {
                    return Ok(path);
                } else {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "Only path fields are supported",
                    ));
                }
            }
        }
    }

    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "Only path fields are supported",
    ))
}

fn get_inner_encryptable_type(ty: syn::Type) -> syn::Result<syn::Ident> {
    if let syn::Type::Path(mut path) = ty {
        while let Ok(p) = get_inner_type(&path) {
            path = p
        }

        return path
            .path
            .segments
            .last()
            .map(|last_path_segment| last_path_segment.ident.to_owned())
            .ok_or(syn::Error::new(
                proc_macro2::Span::call_site(),
                "Atleast one ident must be specified",
            ));
    }

    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "Only path fields are supported",
    ))
}

pub fn derive_to_encryption(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let struct_name = input.ident;
    let fields = get_encryptable_fields(get_struct_fields(input.data)?);

    Ok(generate_encryption_function(struct_name, fields))
}

#[inline(always)]
fn get_encryptable_fields(fields: Punctuated<Field, Comma>) -> Vec<Field> {
    fields
        .into_iter()
        .filter(|field| {
            field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("encrypt"))
        })
        .collect()
}

fn generate_impls(
    gen1: proc_macro2::TokenStream,
    gen2: proc_macro2::TokenStream,
    gen3: proc_macro2::TokenStream,
    impl_st: proc_macro2::TokenStream,
    inner: &[Field],
    ty: Type,
) -> proc_macro2::TokenStream {
    let to_encryptable_impl = inner
        .iter()
        .flat_map(|field| get_field_type(field.ty.clone()).map(|field_ty| (field, field_ty)))
        .map(|(field, field_ty)| {
            let is_option = field_ty.eq("Option");
            let field_ident = &field.ident;
            let field_ident_string = field_ident.as_ref().map(|s| s.to_string());

            if is_option || ty == Type::Updated {
                quote! { self.#field_ident.map(|s| map.insert(#field_ident_string.to_string(),s)) }
            } else {
                quote! { map.insert(#field_ident_string.to_string(),self.#field_ident) }
            }
        });

    let from_encryptable_impl = inner
        .iter()
        .flat_map(|field| get_field_type(field.ty.clone()).map(|field_ty| (field, field_ty)))
        .map(|(field, field_ty)| {
            let is_option = field_ty.eq("Option");
            let field_ident = &field.ident;
            let field_ident_string = field_ident.as_ref().map(|s| s.to_string());

            if is_option || ty == Type::Updated {
                quote! { #field_ident: map.remove(#field_ident_string) }
            } else {
                quote! { #field_ident: map.remove(#field_ident_string).ok_or(
                    error_stack::report!(common_utils::errors::ParsingError::EncodeError(
                        "Unable to convert from HashMap",
                    )),
                )?}
            }
        });

    quote! {
        impl ToEncryptable<#gen1, #gen2, #gen3> for #impl_st {
            fn to_encryptable(self) -> FxHashMap<String, #gen3> {
                let mut map = FxHashMap::with_capacity_and_hasher(3, Default::default());
                #(#to_encryptable_impl;)*
                map
            }

            fn from_encryptable(
                mut map: FxHashMap<String, Encryptable<#gen2>>,
            ) -> CustomResult<#gen1, common_utils::errors::ParsingError> {
                Ok(#gen1 {
                    #(#from_encryptable_impl,)*
                })
          }
        }
    }
}

fn generate_to_encryptable(
    struct_name: syn::Ident,
    fields: Vec<Field>,
) -> proc_macro2::TokenStream {
    let from_request_struct_name = format_ident!("FromRequestEncryptable{}", struct_name);
    let update_struct_name = format_ident!("UpdateEncryptable{}", struct_name);
    let update_decrypted_struct_name = format_ident!("DecryptedUpdate{}", struct_name);
    let encrypted_struct_name = format_ident!("Encrypted{}", struct_name);
    let domain_struct_name = format_ident!("Decrypted{}", struct_name);

    let inner_types = get_field_and_inner_types(&fields);

    let from_request_fields = generate_struct_fields(Type::FromRequest, &inner_types);
    let from_request_struct = quote! {
        pub struct #from_request_struct_name {
            #(#from_request_fields,)*
        }

    };
    let update_fields = generate_struct_fields(Type::Updated, &inner_types);
    let update_struct = quote! {
        pub struct #update_struct_name {
            #(#update_fields,)*
        }
    };

    let update_struct_decrypted_fields =
        generate_struct_fields(Type::DecryptedUpdate, &inner_types);
    let update_struct_decrypted = quote! {
        pub struct #update_decrypted_struct_name {
            #(#update_struct_decrypted_fields,)*
        }
    };
    let encrypted_fields = generate_struct_fields(Type::Encrypted, &inner_types);
    let encrypted_struct = quote! {
        pub struct #encrypted_struct_name {
            #(#encrypted_fields,)*
        }
    };

    let decrypted_fields = generate_struct_fields(Type::Decrypted, &inner_types);
    let decrypted_struct = quote! {
        pub struct #domain_struct_name {
            #(#decrypted_fields,)*
        }
    };

    let inner_type = inner_types.first().unwrap().1.clone();
    let domain_struct_name_clone = domain_struct_name.clone();
    let from_request_struct_name_clone = from_request_struct_name.clone();

    let from_request_struct_impl = generate_impls(
        quote! {#domain_struct_name_clone},
        quote! {Secret<#inner_type>},
        quote! { Secret<#inner_type> },
        quote! { #from_request_struct_name_clone },
        &fields,
        Type::FromRequest,
    );
    let domain_struct_name_clone = domain_struct_name.clone();
    let encrypted_struct_clone = encrypted_struct_name.clone();

    let encrypted_struct_impl = generate_impls(
        quote! {#domain_struct_name_clone},
        quote! {Secret<#inner_type>},
        quote! { Encryption },
        quote! { #encrypted_struct_clone },
        &fields,
        Type::Encrypted,
    );

    let udapte_struct_impl = generate_impls(
        quote! {#update_decrypted_struct_name},
        quote! {Secret<#inner_type>},
        quote! { Secret<#inner_type> },
        quote! { #update_struct_name },
        &fields,
        Type::Updated,
    );

    quote! {
        #from_request_struct
        #update_struct
        #encrypted_struct
        #decrypted_struct
        #update_struct_decrypted
        #from_request_struct_impl
        #encrypted_struct_impl
        #udapte_struct_impl
    }
}

fn get_field_and_inner_types(fields: &[Field]) -> Vec<(Field, syn::Ident)> {
    fields
        .iter()
        .flat_map(|field| {
            get_inner_encryptable_type(field.ty.clone())
                .map(|field_name| (field.to_owned(), field_name))
        })
        .collect()
}

fn generate_struct_fields(
    op_type: Type,
    fields: &[(Field, syn::Ident)],
) -> Vec<proc_macro2::TokenStream> {
    match op_type {
        Type::Encrypted => fields
            .iter()
            .map(|(field, _)| {
                let is_option = get_field_type(field.ty.clone())
                    .map(|f| f.eq("Option"))
                    .unwrap_or_default();

                let ident = &field.ident;

                if is_option {
                    quote! {pub #ident: Option<Encryption>}
                } else {
                    quote! {pub #ident: Encryption}
                }
            })
            .collect(),
        Type::Decrypted => fields
            .iter()
            .map(|(field, inner_ty)| {
                let is_option = get_field_type(field.ty.clone())
                    .map(|f| f.eq("Option"))
                    .unwrap_or_default();
                let ident = &field.ident;

                if is_option {
                    quote! {pub #ident: Option<Encryptable<Secret<#inner_ty>>> }
                } else {
                    quote! {pub #ident: Encryptable<Secret<#inner_ty>> }
                }
            })
            .collect(),
        Type::DecryptedUpdate => fields
            .iter()
            .map(|(field, inner_ty)| {
                let ident = &field.ident;

                quote! {pub #ident: Option<Encryptable<Secret<#inner_ty>>> }
            })
            .collect(),
        Type::FromRequest => fields
            .iter()
            .map(|(field, inner_ty)| {
                let is_option = get_field_type(field.ty.clone())
                    .map(|f| f.eq("Option"))
                    .unwrap_or_default();
                let ident = &field.ident;

                if is_option {
                    quote! {pub #ident: Option<Secret<#inner_ty>> }
                } else {
                    quote! {pub #ident: Secret<#inner_ty> }
                }
            })
            .collect(),
        Type::Updated => fields
            .iter()
            .map(|(field, inner_ty)| {
                let ident = &field.ident;

                quote! {pub #ident: Option<Secret<#inner_ty>> }
            })
            .collect(),
    }
}

fn generate_encryption_function(
    struct_name: syn::Ident,
    fields: Vec<Field>,
) -> proc_macro2::TokenStream {
    //Remove the attributes and collect the field
    let fields = fields
        .into_iter()
        .map(|mut f| {
            f.attrs = Vec::from_iter([syn::parse_quote! {#[serde(default)]}]);
            f
        })
        .collect::<Vec<Field>>();

    let token = generate_to_encryptable(struct_name, fields);

    token
}
