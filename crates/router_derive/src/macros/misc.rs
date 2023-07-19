pub fn get_field_type(field_type: syn::Type) -> Option<syn::Ident> {
    if let syn::Type::Path(path) = field_type {
        path.path.get_ident().cloned()
    } else {
        None
    }
}

/// Implement the `validate` function for the struct by calling `validate` function on the fields
pub fn validate_config(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    let fields = super::helpers::get_struct_fields(input.data)
        .map_err(|error| syn::Error::new(proc_macro2::Span::call_site(), error))?;

    let struct_name = input.ident;
    let function_expansions = fields
        .into_iter()
        .flat_map(|field| field.ident.to_owned().zip(get_field_type(field.ty)))
        .map(|(field_ident, field_type_ident)| {
            // Check if a field is a leaf field, only String ( connector urls ) is supported for now
            let is_leaf_field = field_type_ident.eq("String");
            let field_ident_string = field_ident.to_string();
            if is_leaf_field {
                quote::quote!(common_utils::fp_utils::when(
                        self.#field_ident.is_empty(),
                        || {
                            Err(ApplicationError::InvalidConfigurationValueError(
                                format!("{} must not be empty for {parent_field}", #field_ident_string).into(),
                            ))
                        }
                    )?;
                )
            } else {
                quote::quote!(
                    self.#field_ident.validate(#field_ident_string)?;
                )
            }
        })
        .collect::<Vec<_>>();

    let expansion = quote::quote! {
        impl #struct_name {
            pub fn validate(&self, parent_field: &str) -> Result<(), ApplicationError> {
                #(#function_expansions)*
                Ok(())
            }
        }
    };

    Ok(expansion)
}
