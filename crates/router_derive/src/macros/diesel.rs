use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{parse::Parse, Data, DeriveInput, ItemEnum};

use crate::macros::helpers;

pub(crate) fn diesel_enum_text_derive_inner(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    match &ast.data {
        Data::Enum(_) => (),
        _ => return Err(helpers::non_enum_error()),
    };

    Ok(quote! {

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

    match &ast.data {
        Data::Enum(_) => (),
        _ => return Err(helpers::non_enum_error()),
    };

    let struct_name = format_ident!("Db{name}");
    let type_name = format!("{name}");

    Ok(quote! {

        #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::fmt::Debug, ::diesel::QueryId, ::diesel::SqlType)]
        #[diesel(postgres_type(name = #type_name))]
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

    custom_keyword!(storage_type);
    custom_keyword!(db_enum);
    custom_keyword!(text);
}

#[derive(Debug, strum::EnumString, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum StorageType {
    /// Store the Enum as Text value in the database
    Text,
    /// Store the Enum as Enum in the database. This requires a corresponding enum to be created
    /// in the database with the same name
    DbEnum,
}

#[derive(Debug)]
pub enum DieselEnumMeta {
    StorageTypeEnum {
        keyword: diesel_keyword::storage_type,
        value: StorageType,
    },
}

impl Parse for StorageType {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let text = input.parse::<syn::LitStr>()?;
        let value = text.value();

        value.as_str().parse().map_err(|_| {
            syn::Error::new_spanned(
                &text,
                format!(
                    "Unexpected value for storage_type: `{value}`. Possible values are `{}`",
                    helpers::get_possible_values_for_enum::<Self>()
                ),
            )
        })
    }
}

impl DieselEnumMeta {
    pub fn get_storage_type(&self) -> &StorageType {
        match self {
            Self::StorageTypeEnum { value, .. } => value,
        }
    }
}

impl Parse for DieselEnumMeta {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(diesel_keyword::storage_type) {
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
        helpers::get_metadata_inner("storage_type", &self.attrs)
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
}

/// Based on the storage type, derive appropriate diesel traits
/// This will add the appropriate #[diesel(sql_type)]
/// Since the `FromSql` and `ToSql` have to be derived for all the enums, this will add the
/// `DieselEnum` derive trait.
pub(crate) fn diesel_enum_attribute_macro(
    diesel_enum_meta: DieselEnumMeta,
    item: &ItemEnum,
) -> syn::Result<TokenStream> {
    let diesel_derives =
        quote!(#[derive(diesel::AsExpression, diesel::FromSqlRow, router_derive::DieselEnum) ]);

    match diesel_enum_meta {
        DieselEnumMeta::StorageTypeEnum {
            value: storage_type,
            ..
        } => match storage_type {
            StorageType::Text => Ok(quote! {
                #diesel_derives
                #[diesel(sql_type = ::diesel::sql_types::Text)]
                #[storage_type(storage_type = "text")]
                #item
            }),
            StorageType::DbEnum => {
                let name = &item.ident;
                let type_name = format_ident!("Db{name}");
                Ok(quote! {
                    #diesel_derives
                    #[diesel(sql_type = #type_name)]
                    #[storage_type(storage_type= "db_enum")]
                    #item
                })
            }
        },
    }
}
