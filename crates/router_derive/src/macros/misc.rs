/// Implement the `validate` function for the struct by calling validate function() on the fields
pub fn validate_config(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    let fields = super::helpers::get_struct_fields(input.data)
        .map_err(|error| syn::Error::new(proc_macro2::Span::call_site(), error))?;

    let struct_name = input.ident;
    let function_expansions = fields
        .into_iter()
        .flat_map(|field| field.ident)
        .map(|field_ident| {
            let field_ident_string = field_ident.to_string();
            quote::quote!(
                self.#field_ident.validate(#field_ident_string)?;
            )
        })
        .collect::<Vec<_>>();

    let expansion = quote::quote! {
        impl #struct_name {
            pub fn validate(&self) -> Result<(), ApplicationError> {
                #(#function_expansions)*
                Ok(())
            }
        }
    };

    Ok(expansion)
}
