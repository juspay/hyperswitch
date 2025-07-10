mod helpers;

use quote::quote;

use crate::macros::{
    helpers as macro_helpers,
    schema::helpers::{HasSchemaParameters, IsSchemaFieldApplicableForValidation},
};

pub fn validate_schema_derive(input: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    // Extract struct fields
    let fields = macro_helpers::get_struct_fields(input.data)
        .map_err(|error| syn::Error::new(proc_macro2::Span::call_site(), error))?;

    // Map over each field
    let validation_checks = fields.iter().filter_map(|field| {
        let field_name = field.ident.as_ref()?;
        let field_type = &field.ty;

        // Check if field type is valid for validation
        let is_field_valid = match IsSchemaFieldApplicableForValidation::from(field_type) {
            IsSchemaFieldApplicableForValidation::Invalid => return None,
            val => val,
        };

        // Parse attribute parameters for 'schema'
        let schema_params = match field.get_schema_parameters() {
            Ok(params) => params,
            Err(_) => return None,
        };

        let min_length = schema_params.min_length;
        let max_length = schema_params.max_length;

        // Skip if no length validation is needed
        if min_length.is_none() && max_length.is_none() {
            return None;
        }

        let min_check = min_length.map(|min_val| {
            quote! {
                if value_len < #min_val {
                    return Err(format!("{} must be at least {} characters long. Received {} characters", 
                        stringify!(#field_name), #min_val, value_len));
                }
            }
        }).unwrap_or_else(|| quote! {});

        let max_check = max_length.map(|max_val| {
            quote! {
                if value_len > #max_val {
                    return Err(format!("{} must be at most {} characters long. Received {} characters", 
                        stringify!(#field_name), #max_val, value_len));
                }
            }
        }).unwrap_or_else(|| quote! {});

        // Generate length validation
        if is_field_valid == IsSchemaFieldApplicableForValidation::ValidOptional {
            Some(quote! {
                if let Some(value) = &self.#field_name {
                    let value_len = value.as_str().len();
                    #min_check
                    #max_check
                }
            })
        } else {
            Some(quote! {
                {
                    let value_len = self.#field_name.as_str().len();
                    #min_check
                    #max_check
                }
            })
        }
    }).collect::<Vec<_>>();

    Ok(quote! {
        impl #name {
            pub fn validate(&self) -> Result<(), String> {
                #(#validation_checks)*
                Ok(())
            }
        }
    })
}
