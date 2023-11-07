use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;

fn error() -> TokenStream2 {
    syn::Error::new(
        Span::call_site(),
        "'EnumNums' can only be derived on enums with unit variants".to_string(),
    )
    .to_compile_error()
}

pub(crate) fn enum_nums_inner(ts: TokenStream) -> TokenStream {
    let derive_input = syn::parse_macro_input!(ts as syn::DeriveInput);

    let enum_obj = match derive_input.data {
        syn::Data::Enum(e) => e,
        _ => return error().into(),
    };

    let enum_name = derive_input.ident;

    let mut match_arms = Vec::<TokenStream2>::with_capacity(enum_obj.variants.len());

    for (i, variant) in enum_obj.variants.iter().enumerate() {
        match variant.fields {
            syn::Fields::Unit => {}
            _ => return error().into(),
        }

        let var_ident = &variant.ident;

        match_arms.push(quote! { Self::#var_ident => #i });
    }

    let impl_block = quote! {
        impl #enum_name {
            pub fn to_num(&self) -> usize {
                match self {
                    #(#match_arms),*
                }
            }
        }
    };

    impl_block.into()
}
