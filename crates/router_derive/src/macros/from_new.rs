use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields};

/// Generate an `impl From<EntityNew> for Entity` based on struct fields.
///
/// Assumes the convention:
/// - The target `New` type is `{StructName}New`.
/// - Both `Entity` and `EntityNew` share the same field names and compatible types.
pub fn from_new_derive_inner(input: DeriveInput) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
    let new_type_name = syn::Ident::new(&format!("{}New", struct_name), struct_name.span());

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    struct_name,
                    "FromNew only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                struct_name,
                "FromNew only supports structs",
            ))
        }
    };

    let field_assignments = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().expect("named fields only");
        quote! {
            #field_name: new.#field_name
        }
    });

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        #[automatically_derived]
        impl #impl_generics From<#new_type_name> for #struct_name #ty_generics #where_clause {
            fn from(new: #new_type_name) -> Self {
                Self {
                    #(#field_assignments),*
                }
            }
        }
    };

    Ok(expanded)
}
