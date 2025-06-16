use std::iter::Iterator;

use quote::{format_ident, quote};
use syn::{parse::Parse, punctuated::Punctuated, token::Comma, Field, Ident, Type as SynType};

use crate::macros::{helpers::get_struct_fields, misc::get_field_type};

pub struct FieldMeta {
    _meta_type: Ident,
    pub value: Ident,
}

impl Parse for FieldMeta {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let _meta_type: Ident = input.parse()?;
        input.parse::<syn::Token![=]>()?;
        let value: Ident = input.parse()?;
        Ok(Self { _meta_type, value })
    }
}

impl quote::ToTokens for FieldMeta {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.value.to_tokens(tokens);
    }
}

fn get_encryption_ty_meta(field: &Field) -> Option<FieldMeta> {
    let attrs = &field.attrs;

    attrs
        .iter()
        .flat_map(|s| s.parse_args::<FieldMeta>())
        .find(|s| s._meta_type.eq("ty"))
}

fn get_inner_type(path: &syn::TypePath) -> syn::Result<syn::TypePath> {
    path.path
        .segments
        .last()
        .and_then(|segment| match &segment.arguments {
            syn::PathArguments::AngleBracketed(args) => args.args.first(),
            _ => None,
        })
        .and_then(|arg| match arg {
            syn::GenericArgument::Type(SynType::Path(path)) => Some(path.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "Only path fields are supported",
            )
        })
}

/// This function returns the inner most type recursively
/// For example:
///
/// In the case of `Encryptable<Secret<String>>> this returns String
fn get_inner_most_type(ty: SynType) -> syn::Result<Ident> {
    fn get_inner_type_recursive(path: syn::TypePath) -> syn::Result<syn::TypePath> {
        match get_inner_type(&path) {
            Ok(inner_path) => get_inner_type_recursive(inner_path),
            Err(_) => Ok(path),
        }
    }

    match ty {
        SynType::Path(path) => {
            let inner_path = get_inner_type_recursive(path)?;
            inner_path
                .path
                .segments
                .last()
                .map(|last_segment| last_segment.ident.to_owned())
                .ok_or_else(|| {
                    syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "At least one ident must be specified",
                    )
                })
        }
        _ => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Only path fields are supported",
        )),
    }
}

/// This returns the field which implement #[encrypt] attribute
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

/// This function returns the inner most type of a field
fn get_field_and_inner_types(fields: &[Field]) -> Vec<(Field, Ident)> {
    fields
        .iter()
        .flat_map(|field| {
            get_inner_most_type(field.ty.clone()).map(|field_name| (field.to_owned(), field_name))
        })
        .collect()
}

/// The type of the struct for which the batch encryption/decryption needs to be implemented
#[derive(PartialEq, Copy, Clone)]
enum StructType {
    Encrypted,
    Decrypted,
    DecryptedUpdate,
    FromRequest,
    Updated,
}

impl StructType {
    /// Generates the fields for temporary structs which consists of the fields that should be
    /// encrypted/decrypted
    fn generate_struct_fields(self, fields: &[(Field, Ident)]) -> Vec<proc_macro2::TokenStream> {
        fields
            .iter()
            .map(|(field, inner_ty)| {
                let provided_ty = get_encryption_ty_meta(field);
                let is_option = get_field_type(field.ty.clone())
                    .map(|f| f.eq("Option"))
                    .unwrap_or_default();
                let ident = &field.ident;
                let inner_ty = if let Some(ref ty) = provided_ty {
                    &ty.value
                } else {
                    inner_ty
                };
                match (self, is_option) {
                    (Self::Encrypted, true) => quote! { pub #ident: Option<Encryption> },
                    (Self::Encrypted, false) => quote! { pub #ident: Encryption },
                    (Self::Decrypted, true) => {
                        quote! { pub #ident: Option<Encryptable<Secret<#inner_ty>>> }
                    }
                    (Self::Decrypted, false) => {
                        quote! { pub #ident: Encryptable<Secret<#inner_ty>> }
                    }
                    (Self::DecryptedUpdate, _) => {
                        quote! { pub #ident: Option<Encryptable<Secret<#inner_ty>>> }
                    }
                    (Self::FromRequest, true) => {
                        quote! { pub #ident: Option<Secret<#inner_ty>> }
                    }
                    (Self::FromRequest, false) => quote! { pub #ident: Secret<#inner_ty> },
                    (Self::Updated, _) => quote! { pub #ident: Option<Secret<#inner_ty>> },
                }
            })
            .collect()
    }

    /// Generates the ToEncryptable trait implementation
    fn generate_impls(
        self,
        gen1: proc_macro2::TokenStream,
        gen2: proc_macro2::TokenStream,
        gen3: proc_macro2::TokenStream,
        impl_st: proc_macro2::TokenStream,
        inner: &[Field],
    ) -> proc_macro2::TokenStream {
        let map_length = inner.len();

        let to_encryptable_impl = inner.iter().flat_map(|field| {
            get_field_type(field.ty.clone()).map(|field_ty| {
                let is_option = field_ty.eq("Option");
                let field_ident = &field.ident;
                let field_ident_string = field_ident.as_ref().map(|s| s.to_string());

                if is_option || self == Self::Updated {
                    quote! { self.#field_ident.map(|s| map.insert(#field_ident_string.to_string(), s)) }
                } else {
                    quote! { map.insert(#field_ident_string.to_string(), self.#field_ident) }
                }
            })
        });

        let from_encryptable_impl = inner.iter().flat_map(|field| {
            get_field_type(field.ty.clone()).map(|field_ty| {
                let is_option = field_ty.eq("Option");
                let field_ident = &field.ident;
                let field_ident_string = field_ident.as_ref().map(|s| s.to_string());

                if is_option || self == Self::Updated {
                    quote! { #field_ident: map.remove(#field_ident_string) }
                } else {
                    quote! {
                        #field_ident: map.remove(#field_ident_string).ok_or(
                            error_stack::report!(common_utils::errors::ParsingError::EncodeError(
                                "Unable to convert from HashMap",
                            ))
                        )?
                    }
                }
            })
        });

        quote! {
            impl ToEncryptable<#gen1, #gen2, #gen3> for #impl_st {
                fn to_encryptable(self) -> FxHashMap<String, #gen3> {
                    let mut map = FxHashMap::with_capacity_and_hasher(#map_length, Default::default());
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
}

/// This function generates the temporary struct and ToEncryptable impls for the temporary structs
fn generate_to_encryptable(
    struct_name: Ident,
    fields: Vec<Field>,
) -> syn::Result<proc_macro2::TokenStream> {
    let struct_types = [
        // The first two are to be used as return types we do not need to implement ToEncryptable
        // on it
        ("Decrypted", StructType::Decrypted),
        ("DecryptedUpdate", StructType::DecryptedUpdate),
        ("FromRequestEncryptable", StructType::FromRequest),
        ("Encrypted", StructType::Encrypted),
        ("UpdateEncryptable", StructType::Updated),
    ];

    let inner_types = get_field_and_inner_types(&fields);

    let inner_type = inner_types.first().ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "Please use the macro with attribute #[encrypt] on the fields you want to encrypt",
        )
    })?;

    let provided_ty = get_encryption_ty_meta(&inner_type.0)
        .map(|ty| ty.value.clone())
        .unwrap_or(inner_type.1.clone());

    let structs = struct_types.iter().map(|(prefix, struct_type)| {
        let name = format_ident!("{}{}", prefix, struct_name);
        let temp_fields = struct_type.generate_struct_fields(&inner_types);
        quote! {
            pub struct #name {
                #(#temp_fields,)*
            }
        }
    });

    // These implementations shouldn't be implemented Decrypted and DecryptedUpdate temp structs
    // So skip the first two entries in the list
    let impls = struct_types
        .iter()
        .skip(2)
        .map(|(prefix, struct_type)| {
            let name = format_ident!("{}{}", prefix, struct_name);

            let impl_block = if *struct_type != StructType::DecryptedUpdate
                || *struct_type != StructType::Decrypted
            {
                let (gen1, gen2, gen3) = match struct_type {
                    StructType::FromRequest => {
                        let decrypted_name = format_ident!("Decrypted{}", struct_name);
                        (
                            quote! { #decrypted_name },
                            quote! { Secret<#provided_ty> },
                            quote! { Secret<#provided_ty> },
                        )
                    }
                    StructType::Encrypted => {
                        let decrypted_name = format_ident!("Decrypted{}", struct_name);
                        (
                            quote! { #decrypted_name },
                            quote! { Secret<#provided_ty> },
                            quote! { Encryption },
                        )
                    }
                    StructType::Updated => {
                        let decrypted_update_name = format_ident!("DecryptedUpdate{}", struct_name);
                        (
                            quote! { #decrypted_update_name },
                            quote! { Secret<#provided_ty> },
                            quote! { Secret<#provided_ty> },
                        )
                    }
                    //Unreachable statement
                    _ => (quote! {}, quote! {}, quote! {}),
                };

                struct_type.generate_impls(gen1, gen2, gen3, quote! { #name }, &fields)
            } else {
                quote! {}
            };

            Ok(quote! {
                #impl_block
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #(#structs)*
        #(#impls)*
    })
}

pub fn derive_to_encryption(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let struct_name = input.ident;
    let fields = get_encryptable_fields(get_struct_fields(input.data)?);

    generate_to_encryptable(struct_name, fields)
}
