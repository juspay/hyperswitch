use proc_macro2::Span;
use quote::ToTokens;
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, Attribute, Token};

/// Returns a syntax error with a message indicating that the macro only supports enums.
pub fn non_enum_error() -> syn::Error {
    syn::Error::new(Span::call_site(), "This macro only supports enums.")
}

/// This method creates a syn::Error instance with a message indicating that multiple occurrences of an error attribute were found. It takes two tokens representing the first and second keywords, and a string representing the attribute, and returns the syn::Error instance.
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

/// Creates a new `syn::Error` with the given `span` and `message`.
pub(super) fn syn_error(span: Span, message: &str) -> syn::Error {
    syn::Error::new(span, message)
}

/// Get all the variants of a enum in the form of a string
pub fn get_possible_values_for_enum<T>() -> String
where
    T: strum::IntoEnumIterator + ToString,
{
    T::iter()
        .map(|variants| variants.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// This method takes an identifier and a collection of attributes, filters the attributes based on the identifier, and then parses the filtered attributes into a vector of type T. It returns the parsed vector or an error result.
pub(super) fn get_metadata_inner<'a, T: Parse + Spanned>(
    ident: &str,
    attrs: impl IntoIterator<Item = &'a Attribute>,
) -> syn::Result<Vec<T>> {
    attrs
        .into_iter()
        .filter(|attr| attr.path().is_ident(ident))
        .try_fold(Vec::new(), |mut vec, attr| {
            vec.extend(attr.parse_args_with(Punctuated::<T, Token![,]>::parse_terminated)?);
            Ok(vec)
        })
}

/// This method takes a syn::Data and returns a Result containing a Punctuated list of syn::Field items. 
/// If the input data is a struct with named fields, it returns Ok with a cloned list of the named fields. 
/// If the input data is not a struct with named fields, it returns an Err with an error message.
pub(super) fn get_struct_fields(
    data: syn::Data,
) -> syn::Result<Punctuated<syn::Field, syn::token::Comma>> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = data
    {
        Ok(named.to_owned())
    } else {
        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "This macro cannot be used on structs with no fields",
        ))
    }
}
