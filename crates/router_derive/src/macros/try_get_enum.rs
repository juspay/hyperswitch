use proc_macro2::Span;
use quote::ToTokens;
use syn::{parse::Parse, punctuated::Punctuated};

mod try_get_keyword {
    use syn::custom_keyword;

    custom_keyword!(error_type);
}

#[derive(Debug)]
pub struct TryGetEnumMeta {
    error_type: syn::Ident,
    variant: syn::Ident,
}

impl Parse for TryGetEnumMeta {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let error_type = input.parse()?;
        _ = input.parse::<syn::Token![::]>()?;
        let variant = input.parse()?;

        Ok(Self {
            error_type,
            variant,
        })
    }
}

trait TryGetDeriveInputExt {
    /// Get all the error metadata associated with an enum.
    fn get_metadata(&self) -> syn::Result<Vec<TryGetEnumMeta>>;
}

impl TryGetDeriveInputExt for syn::DeriveInput {
    fn get_metadata(&self) -> syn::Result<Vec<TryGetEnumMeta>> {
        super::helpers::get_metadata_inner("error", &self.attrs)
    }
}

impl ToTokens for TryGetEnumMeta {
    fn to_tokens(&self, _: &mut proc_macro2::TokenStream) {}
}

/// Try and get the variants for an enum
pub fn try_get_enum_variant(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let name = &input.ident;
    let parsed_error_type = input.get_metadata()?;

    let (error_type, error_variant) = parsed_error_type
        .first()
        .ok_or(syn::Error::new(
            Span::call_site(),
            "One error should be specified",
        ))
        .map(|error_struct| (&error_struct.error_type, &error_struct.variant))?;

    let (impl_generics, generics, where_clause) = input.generics.split_for_impl();

    let variants = get_enum_variants(&input.data)?;

    let try_into_fns = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_field = get_enum_variant_field(variant)?;
        let variant_types = variant_field.iter().map(|f|f.ty.clone());

        let try_into_fn = syn::Ident::new(
            &format!("try_into_{}", variant_name.to_string().to_lowercase()),
            Span::call_site(),
        );

        Ok(quote::quote! {
            pub fn #try_into_fn(self)->Result<(#(#variant_types),*),error_stack::Report<#error_type>> {
                match self {
                    Self::#variant_name(inner) => Ok(inner),
                    _=> Err(error_stack::report!(#error_type::#error_variant)),
                }
            }
        })
    }).collect::<Result<Vec<proc_macro2::TokenStream>,syn::Error>>()?;

    let expanded = quote::quote! {
        impl #impl_generics #name #generics #where_clause {
            #(#try_into_fns)*
        }
    };

    Ok(expanded)
}

/// Get variants from Enum
fn get_enum_variants(data: &syn::Data) -> syn::Result<Punctuated<syn::Variant, syn::token::Comma>> {
    if let syn::Data::Enum(syn::DataEnum { variants, .. }) = data {
        Ok(variants.clone())
    } else {
        Err(super::helpers::non_enum_error())
    }
}

/// Get Field from an enum variant
fn get_enum_variant_field(
    variant: &syn::Variant,
) -> syn::Result<Punctuated<syn::Field, syn::token::Comma>> {
    let field = match variant.fields.clone() {
        syn::Fields::Unnamed(un) => un.unnamed,
        syn::Fields::Named(n) => n.named,
        syn::Fields::Unit => {
            return Err(super::helpers::syn_error(
                Span::call_site(),
                "The enum is a unit variant it's not supported",
            ))
        }
    };
    Ok(field)
}
