#![allow(clippy::use_self)]
use std::str::FromStr;

use darling::FromMeta;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use strum::IntoEnumIterator;
use syn::{custom_keyword, parse::Parse, Data, DeriveInput, ItemEnum, TypePath};

use crate::macros::helpers::{get_metadata_inner, non_enum_error};

pub(crate) fn diesel_enum_text_derive_inner(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let variants_data = match &ast.data {
        Data::Enum(v) => &v.variants,
        _ => return Err(non_enum_error()),
    };

    let variants = variants_data
        .into_iter()
        .map(|v| {
            let ident = v.ident.clone();
            quote!(#ident)
        })
        .collect::<Vec<_>>();

    let type_name = format!("InternalEnum{name}");

    let diesel_enum = quote!({
        #[derive(diesel::AsExpression, diesel::FromSqlRow)]
        #[derive(strum::Display, strum::EnumString)]
         #[diesel(sql_type = ::diesel::sql_types::Text)]
        pub enum #type_name {
            #(#variants),*
        }
    });

    Ok(quote! {

        #diesel_enum

        #[automatically_derived]
        impl #impl_generics ::diesel::serialize::ToSql<::diesel::sql_types::Text, ::diesel::pg::Pg> for #name #ty_generics
        #where_clause
        {
            fn to_sql<'b>(&'b self, out: &mut ::diesel::serialize::Output<'b, '_, ::diesel::pg::Pg>) -> ::diesel::serialize::Result {
                use ::std::io::Write;

                out.write_all(self.to_string().as_bytes())?;
                Ok(::diesel::serialize::IsNull::No)
            }
        }

        #[automatically_derived]
        impl #impl_generics ::diesel::deserialize::FromSql<::diesel::sql_types::Text, ::diesel::pg::Pg> for #name #ty_generics
        #where_clause
        {
            fn from_sql(value: ::diesel::pg::PgValue) -> diesel::deserialize::Result<Self> {
                use ::core::str::FromStr;

                Self::from_str(::core::str::from_utf8(value.as_bytes())?)
                    .map_err(|_| "Unrecognized enum variant".into())
            }
        }
    })
}

pub(crate) fn diesel_enum_db_enum_derive_inner(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let variants_data = match &ast.data {
        Data::Enum(v) => &v.variants,
        _ => return Err(non_enum_error()),
    };

    let variants = variants_data
        .into_iter()
        .map(|v| {
            let ident = v.ident.clone();
            quote!(#ident)
        })
        .collect::<Vec<_>>();

    let struct_name = format_ident!("Db{name}");
    dbg!(&struct_name);
    let type_name = format_ident!("InternalEnum{name}");
    let type_name_string = format!("InternalEnum{name}");

    let diesel_enum = quote!(
    #[derive(diesel::AsExpression, diesel::FromSqlRow, Debug)]
    #[derive(strum::Display, strum::EnumString)]
    #[diesel(sql_type = #struct_name)]
    pub enum #type_name {
        #(#variants),*
    });

    Ok(quote! {

        #diesel_enum

        #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::fmt::Debug, ::diesel::QueryId, ::diesel::SqlType)]
        #[diesel(postgres_type(name = #type_name_string))]
        pub struct #struct_name;

        #[automatically_derived]
        impl #impl_generics ::diesel::serialize::ToSql<#struct_name, ::diesel::pg::Pg> for #name #ty_generics
        #where_clause
        {
            fn to_sql<'b>(&'b self, out: &mut ::diesel::serialize::Output<'b, '_, ::diesel::pg::Pg>) -> ::diesel::serialize::Result {
                use ::std::io::Write;

                out.write_all(self.to_string().as_bytes())?;
                Ok(::diesel::serialize::IsNull::No)
            }
        }

        #[automatically_derived]
        impl #impl_generics ::diesel::deserialize::FromSql<#struct_name, ::diesel::pg::Pg> for #name #ty_generics
        #where_clause
        {
            fn from_sql(value: ::diesel::pg::PgValue) -> diesel::deserialize::Result<Self> {
                use ::core::str::FromStr;

                Self::from_str(::core::str::from_utf8(value.as_bytes())?)
                    .map_err(|_| "Unrecognized enum variant".into())
            }
        }
    })
}

mod diesel_keyword {
    use syn::custom_keyword;

    custom_keyword!(db_type);
    custom_keyword!(db_enum);
    custom_keyword!(text);
}

#[derive(Debug, strum::EnumString, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum StorageType {
    Text,
    DbEnum,
}

#[derive(Debug)]
pub enum DieselEnumMeta {
    StorageTypeEnum {
        keyword: diesel_keyword::db_type,
        value: Ident,
    },
}

impl DieselEnumMeta {
    pub fn get_storage_type(&self) -> StorageType {
        match self {
            DieselEnumMeta::StorageTypeEnum { value, .. } => {
                StorageType::from_str(&value.to_string()).unwrap()
            }
        }
    }
}

impl Parse for DieselEnumMeta {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(diesel_keyword::db_type) {
            let keyword = input.parse()?;
            input.parse::<syn::Token![=]>()?;
            let value = input.parse()?;
            Ok(Self::StorageTypeEnum { keyword, value })
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for DieselEnumMeta {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::StorageTypeEnum { keyword, .. } => keyword.to_tokens(tokens),
        }
    }
}

trait DieselDeriveInputExt {
    /// Get all the error metadata associated with an enum.
    fn get_metadata(&self) -> syn::Result<Vec<DieselEnumMeta>>;
}

impl DieselDeriveInputExt for DeriveInput {
    fn get_metadata(&self) -> syn::Result<Vec<DieselEnumMeta>> {
        get_metadata_inner("storage_type", &self.attrs)
    }
}

pub(crate) fn diesel_enum_derive_inner(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let storage_type = ast.get_metadata()?;

    match storage_type
        .first()
        .ok_or(syn::Error::new(
            Span::call_site(),
            "Storage type must be specified",
        ))?
        .get_storage_type()
    {
        StorageType::Text => diesel_enum_text_derive_inner(ast),
        StorageType::DbEnum => diesel_enum_db_enum_derive_inner(ast),
    }

    // Ok(quote! {

    //     #diesel_enum

    //     #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::fmt::Debug, ::diesel::QueryId, ::diesel::SqlType)]
    //     #[diesel(postgres_type(name = #type_name))]
    //     pub struct #struct_name;

    //     #[automatically_derived]
    //     impl #impl_generics ::diesel::serialize::ToSql<#struct_name, ::diesel::pg::Pg> for #name #ty_generics
    //     #where_clause
    //     {
    //         fn to_sql<'b>(&'b self, out: &mut ::diesel::serialize::Output<'b, '_, ::diesel::pg::Pg>) -> ::diesel::serialize::Result {
    //             use ::std::io::Write;

    //             out.write_all(self.to_string().as_bytes())?;
    //             Ok(::diesel::serialize::IsNull::No)
    //         }
    //     }

    //     #[automatically_derived]
    //     impl #impl_generics ::diesel::deserialize::FromSql<#struct_name, ::diesel::pg::Pg> for #name #ty_generics
    //     #where_clause
    //     {
    //         fn from_sql(value: ::diesel::pg::PgValue) -> diesel::deserialize::Result<Self> {
    //             use ::core::str::FromStr;

    //             Self::from_str(::core::str::from_utf8(value.as_bytes())?)
    //                 .map_err(|_| "Unrecognized enum variant".into())
    //         }
    //     }
    // })
}

// pub(crate) fn diesel_enum_attribute_inner(
//     args: &AttributeArgs,
//     item: &ItemEnum,
// ) -> syn::Result<TokenStream> {
//     #[derive(FromMeta, Debug)]
//     enum StorageType {
//         PgEnum,
//         Text,
//     }

//     #[derive(FromMeta, Debug)]
//     struct StorageTypeArgs {
//         storage_type: StorageType,
//     }

//     let storage_type_args = match StorageTypeArgs::from_list(args) {
//         Ok(v) => v,
//         Err(_) => {
//             return Err(syn::Error::new(
//                 Span::call_site(),
//                 "Expected storage_type of text or pg_enum",
//             ));
//         }
//     };

//     match storage_type_args.storage_type {
//         StorageType::PgEnum => {
//             let name = &item.ident;
//             let type_name = format_ident!("Db{name}");
//             Ok(quote! {
//                 #[derive(diesel::AsExpression, diesel::FromSqlRow, router_derive::DieselEnum) ]
//                 #[diesel(sql_type = #type_name)]
//                 #item
//             })
//         }
//         StorageType::Text => Ok(quote! {
//             #[derive(diesel::AsExpression, diesel::FromSqlRow, router_derive::DieselEnumText) ]
//             #[diesel(sql_type = ::diesel::sql_types::Text)]
//             #item
//         }),
//     }
// }
