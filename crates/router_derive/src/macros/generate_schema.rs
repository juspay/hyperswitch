use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use syn::{self, parse::Parse, parse_quote, punctuated::Punctuated, Token};

use crate::macros::helpers;

/// Parse schemas from attribute
/// Example
///
/// #[mandatory_in(PaymentsCreateRequest, PaymentsUpdateRequest)]
/// would return
///
/// [PaymentsCreateRequest, PaymentsUpdateRequest]
fn get_inner_path_ident(attribute: &syn::Attribute) -> syn::Result<Vec<syn::Ident>> {
    Ok(attribute
        .parse_args_with(Punctuated::<syn::Ident, Token![,]>::parse_terminated)?
        .into_iter()
        .collect::<Vec<_>>())
}

#[allow(dead_code)]
/// Get the type of field
fn get_field_type(field_type: syn::Type) -> syn::Result<syn::Ident> {
    if let syn::Type::Path(path) = field_type {
        path.path
            .segments
            .last()
            .map(|last_path_segment| last_path_segment.ident.to_owned())
            .ok_or(syn::Error::new(
                proc_macro2::Span::call_site(),
                "Atleast one ident must be specified",
            ))
    } else {
        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Only path fields are supported",
        ))
    }
}

#[allow(dead_code)]
/// Get the inner type of option
fn get_inner_option_type(field: &syn::Type) -> syn::Result<syn::Ident> {
    if let syn::Type::Path(ref path) = &field {
        if let Some(segment) = path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(ref args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(ty)) = args.args.first() {
                    return get_field_type(ty.clone());
                }
            }
        }
    }

    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "Only path fields are supported",
    ))
}

mod schema_keyword {
    use syn::custom_keyword;

    custom_keyword!(schema);
}

#[derive(Debug, Clone)]
pub struct SchemaMeta {
    struct_name: syn::Ident,
    type_ident: syn::Ident,
}

/// parse #[mandatory_in(PaymentsCreateRequest = u64)]
impl Parse for SchemaMeta {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let struct_name = input.parse::<syn::Ident>()?;
        input.parse::<syn::Token![=]>()?;
        let type_ident = input.parse::<syn::Ident>()?;
        Ok(Self {
            struct_name,
            type_ident,
        })
    }
}

impl quote::ToTokens for SchemaMeta {
    fn to_tokens(&self, _: &mut proc_macro2::TokenStream) {}
}

pub fn polymorphic_macro_derive_inner(
    input: syn::DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    let schemas_to_create =
        helpers::get_metadata_inner::<syn::Ident>("generate_schemas", &input.attrs)?;

    let fields = helpers::get_struct_fields(input.data)
        .map_err(|error| syn::Error::new(proc_macro2::Span::call_site(), error))?;

    // Go through all the fields and create a mapping of required fields for a schema
    // PaymentsCreate -> ["amount","currency"]
    // This will be stored in the hashmap with key as
    // required_fields -> ((amount, PaymentsCreate), (currency, PaymentsCreate))
    // and values as the type
    //
    // (amount, PaymentsCreate) -> Amount
    let mut required_fields = HashMap::<(syn::Ident, syn::Ident), syn::Ident>::new();

    // These fields will be removed in the schema
    // PaymentsUpdate -> ["client_secret"]
    // This will be stored in a hashset
    // hide_fields -> ((client_secret, PaymentsUpdate))
    let mut hide_fields = HashSet::<(syn::Ident, syn::Ident)>::new();
    let mut all_fields = IndexMap::<syn::Field, Vec<syn::Attribute>>::new();

    for field in fields {
        // Partition the attributes of a field into two vectors
        // One with #[mandatory_in] attributes present
        // Rest of the attributes ( include only the schema attribute, serde is not required)
        let (mandatory_attribute, other_attributes) = field
            .attrs
            .iter()
            .partition::<Vec<_>, _>(|attribute| attribute.path().is_ident("mandatory_in"));

        let hidden_fields = field
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("remove_in"))
            .collect::<Vec<_>>();

        // Other attributes ( schema ) are to be printed as is
        other_attributes
            .iter()
            .filter(|attribute| {
                attribute.path().is_ident("schema") || attribute.path().is_ident("doc")
            })
            .for_each(|attribute| {
                // Since attributes will be modified, the field should not contain any attributes
                // So create a field, with previous attributes removed
                let mut field_without_attributes = field.clone();
                field_without_attributes.attrs.clear();

                all_fields
                    .entry(field_without_attributes.to_owned())
                    .or_default()
                    .push(attribute.to_owned().to_owned());
            });

        // Mandatory attributes are to be inserted into hashset
        // The hashset will store it in this format
        // ("amount", PaymentsCreateRequest)
        // ("currency", PaymentsConfirmRequest)
        //
        // For these attributes, we need to later add #[schema(required = true)] attribute
        let field_ident = field.ident.ok_or(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Cannot use `mandatory_in` on unnamed fields",
        ))?;

        // Parse the  #[mandatory_in(PaymentsCreateRequest = u64)] and insert into hashmap
        // key -> ("amount", PaymentsCreateRequest)
        // value -> u64
        if let Some(mandatory_in_attribute) =
            helpers::get_metadata_inner::<SchemaMeta>("mandatory_in", mandatory_attribute)?.first()
        {
            let key = (
                field_ident.clone(),
                mandatory_in_attribute.struct_name.clone(),
            );
            let value = mandatory_in_attribute.type_ident.clone();
            required_fields.insert(key, value);
        }

        // Hidden fields are to be inserted in the Hashset
        // The hashset will store it in this format
        // ("client_secret", PaymentsUpdate)
        //
        // These fields will not be added to the struct
        _ = hidden_fields
            .iter()
            // Filter only #[mandatory_in] attributes
            .map(|&attribute| get_inner_path_ident(attribute))
            .try_for_each(|schemas| {
                let res = schemas
                    .map_err(|error| syn::Error::new(proc_macro2::Span::call_site(), error))?
                    .iter()
                    .map(|schema| (field_ident.clone(), schema.to_owned()))
                    .collect::<HashSet<_>>();

                hide_fields.extend(res);
                Ok::<_, syn::Error>(())
            });
    }

    // iterate over the schemas and build them with their fields
    let schemas = schemas_to_create
        .iter()
        .map(|schema| {
            let fields = all_fields
                .iter()
                .filter_map(|(field, attributes)| {
                    let mut final_attributes = attributes.clone();

                    if let Some(field_ident) = field.ident.to_owned() {
                        // If the field is required for this schema, then add
                        // #[schema(value_type = type)] for this field
                        if let Some(required_field_type) =
                            required_fields.get(&(field_ident, schema.to_owned()))
                        {
                            // This is a required field in the Schema
                            // Add the value type and remove original value type ( if present )
                            let attribute_without_schema_type = attributes
                                .iter()
                                .filter(|attribute| !attribute.path().is_ident("schema"))
                                .map(Clone::clone)
                                .collect::<Vec<_>>();

                            final_attributes = attribute_without_schema_type;

                            let value_type_attribute: syn::Attribute =
                                parse_quote!(#[schema(value_type = #required_field_type)]);
                            final_attributes.push(value_type_attribute);
                        }
                    }

                    // If the field is to be not shown then
                    let is_hidden_field = field
                        .ident
                        .clone()
                        .map(|field_ident| hide_fields.contains(&(field_ident, schema.to_owned())))
                        .unwrap_or(false);

                    if is_hidden_field {
                        None
                    } else {
                        Some(quote::quote! {
                            #(#final_attributes)*
                            #field,
                        })
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
