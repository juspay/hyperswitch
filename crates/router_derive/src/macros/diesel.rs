use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{AttributeArgs, Data, DeriveInput, ItemEnum};

use crate::macros::helpers::non_enum_error;

pub(crate) fn diesel_enum_derive_inner(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    match &ast.data {
        Data::Enum(_) => (),
        _ => return Err(non_enum_error()),
    }

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

pub(crate) fn diesel_enum_attribute_inner(
    args: &AttributeArgs,
    item: &ItemEnum,
) -> syn::Result<TokenStream> {
    if !args.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "This attribute macro does not accept any arguments.",
        ));
    }

    let name = &item.ident;
    let struct_name = format_ident!("Db{name}");
    Ok(quote! {
        #[derive(diesel::AsExpression, diesel::FromSqlRow)]
        #[diesel(sql_type = #struct_name)]
        #item
    })
}
