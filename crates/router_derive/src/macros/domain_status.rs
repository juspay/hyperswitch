//! Attribute macro that turns a plain unit-variant enum into a *domain status*
//! type mirroring a storage status enum, with an internal-only `Unknown`
//! catch-all for connector-response deserialization resilience.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Fields, ItemEnum, Path, Token,
};

/// Parsed arguments for `#[domain_status(storage = <path>)]`.
pub(crate) struct DomainStatusArgs {
    storage: Path,
}

impl Parse for DomainStatusArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        if key != "storage" {
            return Err(syn::Error::new(
                key.span(),
                "expected `storage = <path>` argument",
            ));
        }
        input.parse::<Token![=]>()?;
        let storage: Path = input.parse()?;
        Ok(Self { storage })
    }
}

/// Expand `#[domain_status(storage = S)] enum D { A, B, .. }` into the domain
/// enum (with `#[serde(other)] Unknown`), `From<S> for D`, `TryFrom<D> for S`
/// (erroring on `Unknown`), and `resolve_or_keep` / `is_unknown` / `to_storage`.
pub(crate) fn domain_status_attribute_macro(
    args: DomainStatusArgs,
    item: &ItemEnum,
) -> syn::Result<TokenStream> {
    let storage = &args.storage;
    let vis = &item.vis;
    let ident = &item.ident;
    let enum_attrs = &item.attrs;

    let mut variant_defs = Vec::new();
    let mut variant_idents = Vec::new();
    for variant in &item.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "domain_status only supports unit variants (it mirrors a storage enum)",
            ));
        }
        if variant.ident == "Unknown" {
            return Err(syn::Error::new_spanned(
                variant,
                "do not declare `Unknown` manually; the macro appends it",
            ));
        }
        let variant_attrs = &variant.attrs;
        let variant_ident = &variant.ident;
        variant_defs.push(quote! { #(#variant_attrs)* #variant_ident });
        variant_idents.push(variant_ident.clone());
    }

    let from_arms = variant_idents
        .iter()
        .map(|variant| quote! { <#storage>::#variant => Self::#variant });
    let try_from_arms = variant_idents
        .iter()
        .map(|variant| quote! { #ident::#variant => ::core::result::Result::Ok(Self::#variant) });

    Ok(quote! {
        #(#enum_attrs)*
        #[derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::fmt::Debug,
            ::core::cmp::PartialEq,
            ::core::cmp::Eq,
            ::serde::Serialize,
            ::serde::Deserialize,
        )]
        #[serde(rename_all = "snake_case")]
        #vis enum #ident {
            #(#variant_defs,)*
            /// Connector returned a status we do not model. Internal-only:
            /// must be resolved to the previous state before persistence and is
            /// never written to storage nor sent to merchants.
            #[serde(other)]
            Unknown,
        }

        #[automatically_derived]
        impl ::core::convert::From<#storage> for #ident {
            fn from(value: #storage) -> Self {
                match value {
                    #(#from_arms,)*
                }
            }
        }

        #[automatically_derived]
        impl ::core::convert::TryFrom<#ident> for #storage {
            type Error = common_enums::domain_status::UnknownStatusError;

            fn try_from(value: #ident) -> ::core::result::Result<Self, Self::Error> {
                match value {
                    #(#try_from_arms,)*
                    #ident::Unknown => ::core::result::Result::Err(
                        common_enums::domain_status::UnknownStatusError::new(
                            ::core::stringify!(#ident)
                        )
                    ),
                }
            }
        }

        #[automatically_derived]
        impl #ident {
            /// Replace an `Unknown` status with the previously known storage
            /// state; known statuses pass through unchanged. This is the merge
            /// of connector response and previous state.
            #[must_use]
            pub fn resolve_or_keep(self, previous: #storage) -> Self {
                match self {
                    Self::Unknown => Self::from(previous),
                    known => known,
                }
            }

            /// `true` when the connector status could not be recognised.
            #[must_use]
            pub fn is_unknown(self) -> bool {
                ::core::matches!(self, Self::Unknown)
            }

            /// Convert to the storage representation, erroring if still `Unknown`.
            pub fn to_storage(
                self,
            ) -> ::core::result::Result<#storage, common_enums::domain_status::UnknownStatusError> {
                <#storage as ::core::convert::TryFrom<#ident>>::try_from(self)
            }
        }
    })
}
