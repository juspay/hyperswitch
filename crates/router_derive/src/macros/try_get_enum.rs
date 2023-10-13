use proc_macro2::Span;
use syn::punctuated::Punctuated;

/// Try and get the variants for an enum
pub fn try_get_enum_variant(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let name = &input.ident;

    let error_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path.is_ident("error"))
        .ok_or(super::helpers::syn_error(
            proc_macro2::Span::call_site(),
            "Unable to find attribute error. Expected #[error(..)]",
        ))?;
    let (error_type, error_variant) = get_error_type_and_variant(error_attr)?;
    let (impl_generics, generics, where_clause) = input.generics.split_for_impl();

    let variants = get_enum_variants(&input.data)?;

    let try_into_fns = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_field = get_enum_variant_field(variant)?;
        let variant_types = variant_field.iter().map(|f|f.ty.clone());

        let try_into_fn = syn::Ident::new(
            &format!("try_into_{}", variant_name.to_string().to_lowercase()),
            proc_macro2::Span::call_site(),
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

/// Parses the attribute #[error(ErrorType(ErrorVariant))]
fn get_error_type_and_variant(attr: &syn::Attribute) -> syn::Result<(syn::Ident, syn::Path)> {
    let meta = attr.parse_meta()?;
    let metalist = match meta {
        syn::Meta::List(list) => list,
        _ => {
            return Err(super::helpers::syn_error(
                proc_macro2::Span::call_site(),
                "Invalid attribute format #[error(ErrorType(ErrorVariant)]",
            ))
        }
    };

    for meta in metalist.nested.iter() {
        if let syn::NestedMeta::Meta(syn::Meta::List(meta)) = meta {
            let error_type = meta
                .path
                .get_ident()
                .ok_or(super::helpers::syn_error(
                    proc_macro2::Span::call_site(),
                    "Invalid attribute format #[error(ErrorType(ErrorVariant))]",
                ))
                .cloned()?;
            let error_variant = get_error_variant(meta)?;
            return Ok((error_type, error_variant));
        };
    }

    Err(super::helpers::syn_error(
        proc_macro2::Span::call_site(),
        "Invalid attribute format #[error(ErrorType(ErrorVariant))]",
    ))
}

fn get_error_variant(meta: &syn::MetaList) -> syn::Result<syn::Path> {
    for meta in meta.nested.iter() {
        if let syn::NestedMeta::Meta(syn::Meta::Path(meta)) = meta {
            return Ok(meta.clone());
        }
    }
    Err(super::helpers::syn_error(
        proc_macro2::Span::call_site(),
        "Invalid attribute format expected #[error(ErrorType(ErrorVariant))]",
    ))
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
