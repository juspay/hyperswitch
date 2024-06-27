use crate::macros::{helpers::get_struct_fields, misc::get_field_type};
use quote::{format_ident, quote};
use std::iter::Iterator;
use syn::{punctuated::Punctuated, token::Comma, Field};

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

    let intermediate_struct_name = format_ident!("{}Encryptable", struct_name);

    let to_hashmap = fields
        .iter()
        .flat_map(|field| field.ident.to_owned().zip(get_field_type(field.ty.clone())))
        .map(|(field_name, ty)| {
            let field_name_string = field_name.to_string();

            let is_option = ty.eq("Option");
            if is_option {
                quote! {
                    if let Some(val) = self.#field_name {
                        ser.insert(String::from(#field_name_string), val.switch_strategy());
                    }
                }
            } else {
                quote! {
                    ser.insert(String::from(#field_name_string),self.#field_name.switch_strategy());
                }
            }
        });

    let f = quote! {
        #[automatically_derived]
        #[derive(Deserialize)]
        struct #intermediate_struct_name {
            #(#fields,)*
        }

        #[automatically_derived]
        impl ToEncryptable for #intermediate_struct_name {
            fn to_encryptable(self)-> FxHashMap<String, Secret<String>> {
                use masking::SwitchStrategy;

                let mut ser = FxHashMap::default();
                #(#to_hashmap)*
                ser
            }

            fn from_encryptable(hashmap: FxHashMap<String, Secret<String>>)-> Result<Self, error_stack::Report<errors::ParsingError>> {
                use masking::ExposeInterface;
                use error_stack::ResultExt;

                use serde::de::value::MapDeserializer;

                let hashmap: FxHashMap<String, String> = hashmap.into_iter().map(|(k,v)| (k, v.expose())).collect();

                let iter = MapDeserializer::<
                    '_,
                    std::collections::hash_map::IntoIter<String, String>,
                    serde_json::error::Error,
                >::new(hashmap.into_iter());

                Self::deserialize(iter).change_context(errors::ParsingError::StructParseFailure("Failed to parse the encryptable hashmap to struct"))
            }
        }
    };
    f
}
