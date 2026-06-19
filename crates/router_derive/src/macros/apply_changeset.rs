use quote::quote;
use syn::{parse::Parse, ItemStruct, Path, Token};

/// Parsed contents of `#[apply_changeset(target = TargetType)]`
struct ApplyChangesetAttr {
    target: Path,
}

impl Parse for ApplyChangesetAttr {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        if ident != "target" {
            return Err(syn::Error::new(ident.span(), "expected `target = Type`"));
        }
        input.parse::<Token![=]>()?;
        let target: Path = input.parse()?;
        Ok(Self { target })
    }
}

pub(crate) fn apply_changeset_attribute(
    args: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let apply_attr: ApplyChangesetAttr = syn::parse2(args)?;
    let target_type = apply_attr.target;

    // Parse the annotated struct
    let mut item: ItemStruct = syn::parse2(input)?;
    let struct_name = &item.ident;
    let generics = &item.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Strip the #[apply_changeset(...)] attribute so it isn't re-emitted
    item.attrs
        .retain(|attr| !attr.path().is_ident("apply_changeset"));

    // Extract named fields
    let fields = match &item.fields {
        syn::Fields::Named(named) => named.named.iter().collect::<Vec<_>>(),
        _ => {
            return Err(syn::Error::new(
                item.ident.span(),
                "#[apply_changeset] only supports structs with named fields",
            ))
        }
    };

    let mut assignments = Vec::new();

    for field in &fields {
        let Some(field_name) = field.ident.as_ref() else {
            continue;
        };
        let ty = &field.ty;

        if is_option_type(ty) {
            assignments.push(quote! {
                ::common_utils::ApplyOptionField::apply(
                    self.#field_name, &mut target.#field_name
                );
            });
        } else {
            assignments.push(quote! {
                target.#field_name = self.#field_name;
            });
        }
    }

    Ok(quote! {
        #item

        #[automatically_derived]
        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Applies the fields of this update struct to the given target,
            /// returning the modified target.
            ///
            /// Fields of type `Option<T>` are only applied when `Some`, leaving
            /// the original value unchanged when `None`.
            pub fn apply_changeset(self, mut target: #target_type) -> #target_type {
                #(#assignments)*
                target
            }
        }
    })
}

/// Heuristic: does `ty` look like `Option<T>`?
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}
