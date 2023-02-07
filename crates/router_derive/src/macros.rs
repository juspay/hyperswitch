pub(crate) mod api_error;
pub(crate) mod diesel;
mod helpers;
pub(crate) mod operation;

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub(crate) use self::{
    api_error::api_error_derive_inner,
    diesel::{
        diesel_enum_attribute_inner, diesel_enum_derive_inner, diesel_enum_text_derive_inner,
    },
    operation::operation_derive_inner,
};

pub(crate) fn debug_as_display_inner(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::core::fmt::Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(), ::core::fmt::Error> {
                f.write_str(&format!("{:?}", self))
            }
        }
    })
}
