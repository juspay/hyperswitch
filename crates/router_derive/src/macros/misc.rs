pub fn get_field_type(field_type: syn::Type) -> Option<syn::Ident> {
    if let syn::Type::Path(path) = field_type {
        path.path
            .segments
            .last()
            .map(|last_path_segment| last_path_segment.ident.to_owned())
    } else {
        None
    }
}

#[allow(dead_code)]
/// Get the inner type of option
pub fn get_inner_option_type(field: &syn::Type) -> syn::Result<syn::Ident> {
    if let syn::Type::Path(ref path) = &field {
        if let Some(segment) = path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(ref args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(ty)) = args.args.first() {
                    if let syn::Type::Path(path) = ty.clone() {
                        return path
                            .path
                            .segments
                            .last()
                            .map(|last_path_segment| last_path_segment.ident.to_owned())
                            .ok_or(syn::Error::new(
                                proc_macro2::Span::call_site(),
                                "Atleast one ident must be specified",
                            ));
                    } else {
                        return Err(syn::Error::new(
                            proc_macro2::Span::call_site(),
                            "Only path fields are supported",
                        ));
                    }
                }
            }
        }
    }

    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "Only path fields are supported",
    ))
}

/// Implement the `validate` function for the struct by calling `validate` function on the fields
pub fn validate_config(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    let fields = super::helpers::get_struct_fields(input.data)
        .map_err(|error| syn::Error::new(proc_macro2::Span::call_site(), error))?;

    let struct_name = input.ident;
    let function_expansions = fields
        .into_iter()
        .flat_map(|field| field.ident.to_owned().zip(get_field_type(field.ty)))
        .filter_map(|(field_ident, field_type_ident)| {
            // Check if a field is a leaf field, only String ( connector urls ) is supported for now

            let field_ident_string = field_ident.to_string();
            let is_optional_field = field_type_ident.eq("Option");
            let is_secret_field = field_type_ident.eq("Secret");

            // Do not call validate if it is an optional field
            if !is_optional_field && !is_secret_field {
                let is_leaf_field = field_type_ident.eq("String");
                let validate_expansion = if is_leaf_field {
                quote::quote!(common_utils::fp_utils::when(
                        self.#field_ident.is_empty(),
                        || {
                            Err(ApplicationError::InvalidConfigurationValueError(
                                format!("{} must not be empty for {}", #field_ident_string, parent_field).into(),
                            ))
                        }
                    )?;
                )
                } else {
                    quote::quote!(
                        self.#field_ident.validate(#field_ident_string)?;
                    )
                };
                Some(validate_expansion)
            } else {
                None
            }

        })
        .collect::<Vec<_>>();

    let expansion = quote::quote! {
        impl #struct_name {
            /// Validates that the configuration provided for the `parent_field` does not contain empty or default values
            pub fn validate(&self, parent_field: &str) -> Result<(), ApplicationError> {
                #(#function_expansions)*
                Ok(())
            }
        }
    };

    Ok(expansion)
}
