use std::collections::{HashMap, HashSet};

use syn::{self, parse_quote};

/// For a field, return the schemas where this field has to be marked as mandatory
fn get_ident_from_attr(tokens: proc_macro2::TokenStream) -> Vec<syn::Ident> {
    tokens
        .into_iter()
        .filter_map(|token| {
            if let proc_macro2::TokenTree::Group(token_group) = token {
                // Filter out punctuation marks
                let res = token_group
                    .stream()
                    .into_iter()
                    .filter_map(|token| {
                        if let proc_macro2::TokenTree::Ident(ident) = token {
                            Some(ident)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                Some(res)
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>()
}

/// Parse schemas from #[mandatory_in] attribute
/// Example
///
/// #[mandatory_in(PaymentsCreateRequest, PaymentsUpdateRequest)]
/// would return
///
/// [PaymentsCreateRequest, PaymentsUpdateRequest]
fn get_schemas_to_create(attributes: Vec<syn::Attribute>) -> syn::Result<Vec<syn::Ident>> {
    let attributes = attributes
        .iter()
        .filter(|attribute| {
            attribute
                .path
                .segments
                .first()
                .map(|path_segment| path_segment.ident.to_string().eq("generate_schemas"))
                .unwrap_or(false)
        })
        .flat_map(|attribute| get_ident_from_attr(attribute.tokens.to_owned()))
        .collect::<Vec<_>>();

    if attributes.is_empty() {
        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "At least one schema has to be passed in #[generate_schemas]",
        ))?
    } else {
        Ok(attributes)
    }
}

fn get_struct_fields(
    data: syn::Data,
) -> syn::Result<syn::punctuated::Punctuated<syn::Field, syn::token::Comma>> {
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

pub fn polymorphic_macro_derive_inner(
    input: syn::DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    let schemas_to_create = get_schemas_to_create(input.attrs)
        .map_err(|error| syn::Error::new(proc_macro2::Span::call_site(), error))?;

    let fields = get_struct_fields(input.data)
        .map_err(|error| syn::Error::new(proc_macro2::Span::call_site(), error))?;

    // Go through all the fields and create a mapping of required fields for a schema
    // PaymentsCreate -> ["amount","currency"]
    // This will be stored in a hashset
    // mandatory_hashset -> ((PaymentsCreate, amount), (PaymentsCreate,currency))

    let mut mandatory_hashset = HashSet::<(syn::Ident, syn::Ident)>::new();
    let mut other_fields_hm = HashMap::<syn::Field, Vec<syn::Attribute>>::new();

    fields.iter().for_each(|field| {
        // Partition the attributes of a field into two vectors
        // One with #[mandatory_in] attributes present
        // Rest of the attributes ( include only the schema attribute, serde is not required)
        let (mandatory_attribute, other_attributes) =
            field.attrs.iter().partition::<Vec<_>, _>(|attribute| {
                attribute
                    .path
                    .segments
                    .to_owned()
                    .into_iter()
                    .any(|path_segment| path_segment.ident.to_string().eq("mandatory_in"))
            });

        // Other attributes ( schema ) are to be printed as is
        other_attributes
            .iter()
            .filter(|attribute| {
                attribute
                    .path
                    .segments
                    .to_owned()
                    .into_iter()
                    .any(|path_segment| path_segment.ident.to_string().eq("schema"))
            })
            .for_each(|attribute| {
                // Since attributes will be modified, the field should not contain any attributes
                // So create a field, with previous attributes removed
                let mut field_without_attributes = field.clone();
                field_without_attributes.attrs.clear();

                other_fields_hm
                    .entry(field_without_attributes.to_owned())
                    .or_insert(vec![])
                    .push(attribute.to_owned().to_owned());
            });

        // Mandatory attributes are to be inserted into hashset
        // The hashset will store it in this format
        // (PaymentsCreateRequest, "amount")
        // (PaymentsConfirmRequest, "currency")
        //
        // For these attributes, we need to later add #[schema(required = true)] attribute
        mandatory_attribute
            .iter()
            // Filter only #[mandatory_in] attributes
            .map(|attribute| get_ident_from_attr(attribute.tokens.to_owned()))
            .for_each(|schemas| {
                let res = schemas
                    .iter()
                    .filter_map(|schema| field.ident.to_owned().zip(Some(schema.to_owned())))
                    .collect::<HashSet<_>>();

                mandatory_hashset.extend(res);
            });
    });

    let schemas = schemas_to_create
        .iter()
        .map(|schema| {
            let fields = other_fields_hm
                .iter()
                .flat_map(|(field, value)| {
                    let mut attributes = value
                        .iter()
                        .map(|attribute| quote::quote!(#attribute))
                        .collect::<Vec<_>>();

                    // If the field is required for this schema, then add
                    // #[schema(required = true)] for this field
                    let required_attribute: syn::Attribute =
                        parse_quote!(#[schema(required = true)]);

                    // Can be none, because tuple fields have no ident
                    field.ident.to_owned().and_then(|field_ident| {
                        mandatory_hashset
                            .contains(&(field_ident, schema.to_owned()))
                            .then(|| attributes.push(quote::quote!(#required_attribute)))
                    });

                    quote::quote! {
                        #(#attributes)*
                        #field,
                    }
                })
                .collect::<Vec<_>>();
            quote::quote! {
                #[derive(utoipa::ToSchema)]
                pub struct #schema {
                    #(#fields)*
                }
            }
        })
        .collect::<Vec<_>>();

    Ok(quote::quote! {
        #(#schemas)*
    })
}
