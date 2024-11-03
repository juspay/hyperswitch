use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    braced, bracketed,
    parse::{Parse, ParseBuffer, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
    Ident, Token,
};

struct ResourceInput {
    resource_name: Ident,
    scopes: Punctuated<Ident, Token![,]>,
    entities: Punctuated<Ident, Token![,]>,
}

struct Input {
    permissions: Punctuated<ResourceInput, Token![,]>,
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let (_permission_label, permissions) = parse_label_with_punctuated_data(input)?;

        Ok(Self { permissions })
    }
}

impl Parse for ResourceInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let resource_name: Ident = input.parse()?;
        input.parse::<Token![:]>()?; // Expect ':'

        let content;
        braced!(content in input);

        let (_scopes_label, scopes) = parse_label_with_punctuated_data(&content)?;
        content.parse::<Comma>()?;

        let (_entities_label, entities) = parse_label_with_punctuated_data(&content)?;

        Ok(Self {
            resource_name,
            scopes,
            entities,
        })
    }
}

fn parse_label_with_punctuated_data<T: Parse>(
    input: &ParseBuffer<'_>,
) -> syn::Result<(Ident, Punctuated<T, Token![,]>)> {
    let label: Ident = input.parse()?;
    input.parse::<Token![:]>()?; // Expect ':'

    let content;
    bracketed!(content in input); // Parse the list inside []
    let data = Punctuated::<T, Token![,]>::parse_terminated(&content)?;

    Ok((label, data))
}

pub fn generate_permissions_inner(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Input);

    let res = input.permissions.iter();

    let mut enum_keys = Vec::new();
    let mut scope_impl_per = Vec::new();
    let mut entity_impl_per = Vec::new();
    let mut resource_impl_per = Vec::new();

    let mut entity_impl_res = Vec::new();

    for per in res {
        let resource_name = &per.resource_name;
        let mut permissions = Vec::new();

        for scope in per.scopes.iter() {
            for entity in per.entities.iter() {
                let key = format_ident!("{}{}{}", entity, per.resource_name, scope);

                enum_keys.push(quote! { #key });
                scope_impl_per.push(quote! { Permission::#key => PermissionScope::#scope });
                entity_impl_per.push(quote! { Permission::#key => EntityType::#entity });
                resource_impl_per.push(quote! { Permission::#key => Resource::#resource_name });
                permissions.push(quote! { Permission::#key });
            }
            let entities_iter = per.entities.iter();
            entity_impl_res
                .push(quote! { Resource::#resource_name => vec![#(EntityType::#entities_iter),*] });
        }
    }

    let expanded = quote! {
        #[derive(
            Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, serde::Serialize, serde::Deserialize, strum::Display
        )]
        pub enum Permission {
            #(#enum_keys),*
        }

        impl Permission {
            pub fn scope(&self) -> PermissionScope {
                match self {
                    #(#scope_impl_per),*
                }
            }
            pub fn entity_type(&self) -> EntityType {
                match self {
                    #(#entity_impl_per),*
                }
            }
            pub fn resource(&self) -> Resource {
                match self {
                    #(#resource_impl_per),*
                }
            }
        }

        pub trait ResourceExt {
            fn entities(&self) -> Vec<EntityType>;
        }

        impl ResourceExt for Resource {
            fn entities(&self) -> Vec<EntityType> {
                match self {
                    #(#entity_impl_res),*
                }
            }
        }
    };
    expanded.into()
}
