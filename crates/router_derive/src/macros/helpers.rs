use proc_macro2::Span;
use quote::ToTokens;
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, Attribute, Token};

pub fn non_enum_error() -> syn::Error {
    syn::Error::new(Span::call_site(), "This macro only supports enums.")
}

pub(super) fn occurrence_error<T: ToTokens>(
    first_keyword: T,
    second_keyword: T,
    attr: &str,
) -> syn::Error {
    let mut error = syn::Error::new_spanned(
        second_keyword,
        format!("Found multiple occurrences of error({attr})"),
    );
    error.combine(syn::Error::new_spanned(first_keyword, "first one here"));
    error
}

pub(super) fn syn_error(span: Span, message: &str) -> syn::Error {
    syn::Error::new(span, message)
}

pub(super) fn get_metadata_inner<'a, T: Parse + Spanned>(
    ident: &str,
    attrs: impl IntoIterator<Item = &'a Attribute>,
) -> syn::Result<Vec<T>> {
    attrs
        .into_iter()
        .filter(|attr| attr.path.is_ident(ident))
        .try_fold(Vec::new(), |mut vec, attr| {
            vec.extend(attr.parse_args_with(Punctuated::<T, Token![,]>::parse_terminated)?);
            Ok(vec)
        })
}
