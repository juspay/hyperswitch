use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemTrait, Pat, ReturnType, TraitItem, Type};

/// Generate a caller-tracked interface from an async trait without changing its implementations.
pub(crate) fn track_async_interface(item: ItemTrait) -> syn::Result<TokenStream> {
    let raw_name = item.ident.clone();
    let tracked_name = format_ident!("Tracked{raw_name}");
    let visibility = item.vis.clone();
    let tracked_lifetime = syn::Lifetime::new("'__tracked", proc_macro2::Span::call_site());

    let mut tracked_methods = Vec::new();
    let mut implementations = Vec::new();
    let mut has_error_type = false;

    for trait_item in &item.items {
        let TraitItem::Fn(method) = trait_item else {
            if let TraitItem::Type(associated_type) = trait_item {
                if associated_type.ident == "Error" {
                    has_error_type = true;
                    continue;
                }
            }
            return Err(syn::Error::new_spanned(
                trait_item,
                "only async methods and the Error associated type are supported",
            ));
        };

        let signature = &method.sig;
        if signature.asyncness.is_none()
            || !signature.generics.params.is_empty()
            || signature.generics.where_clause.is_some()
            || method.default.is_some()
        {
            return Err(syn::Error::new_spanned(
                signature,
                "tracked interface methods must be async, non-generic, and required",
            ));
        }

        let ReturnType::Type(_, output) = &signature.output else {
            return Err(syn::Error::new_spanned(
                signature,
                "tracked interface methods must have a return type",
            ));
        };

        let mut inputs = signature.inputs.clone();
        let mut arguments = Vec::new();
        let mut has_receiver = false;
        for input in &mut inputs {
            match input {
                FnArg::Receiver(receiver) => {
                    has_receiver = true;
                    let Some((_, lifetime)) = &mut receiver.reference else {
                        return Err(syn::Error::new_spanned(
                            receiver,
                            "tracked interface methods must take &self",
                        ));
                    };
                    if lifetime.is_some() {
                        return Err(syn::Error::new_spanned(
                            receiver,
                            "explicit receiver lifetimes are not supported",
                        ));
                    }
                    *lifetime = Some(tracked_lifetime.clone());
                    arguments.push(quote!(self));
                }
                FnArg::Typed(argument) => {
                    if let Type::Reference(reference) = argument.ty.as_mut() {
                        if reference.lifetime.is_some() {
                            return Err(syn::Error::new_spanned(
                                reference,
                                "explicit argument lifetimes are not supported",
                            ));
                        }
                        reference.lifetime = Some(tracked_lifetime.clone());
                    }
                    let Pat::Ident(identifier) = argument.pat.as_ref() else {
                        return Err(syn::Error::new_spanned(
                            &argument.pat,
                            "tracked interface arguments must be identifiers",
                        ));
                    };
                    let name = &identifier.ident;
                    arguments.push(quote!(#name));
                }
            }
        }
        if !has_receiver {
            return Err(syn::Error::new_spanned(
                signature,
                "tracked interface methods must take &self",
            ));
        }

        let method_name = &signature.ident;
        let attributes = &method.attrs;
        let future_output = quote!(
            ::std::pin::Pin<
                ::std::boxed::Box<
                    dyn ::std::future::Future<Output = #output> + Send + #tracked_lifetime
                >
            >
        );

        tracked_methods.push(quote! {
            #(#attributes)*
            #[track_caller]
            fn #method_name<#tracked_lifetime>(#inputs) -> #future_output;
        });
        implementations.push(quote! {
            #(#attributes)*
            #[track_caller]
            fn #method_name<#tracked_lifetime>(#inputs) -> #future_output {
                let caller = ::std::panic::Location::caller();
                ::std::boxed::Box::pin(router_env::call_origin::with_origin(
                    caller,
                    <T as #raw_name>::#method_name(#(#arguments),*),
                ))
            }
        });
    }

    // Keep the original trait available to existing store implementations. The generated
    // interface deliberately does not inherit it, so trait objects expose only tracked calls.
    if !has_error_type {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "tracked async interfaces must have an Error associated type",
        ));
    }

    Ok(quote! {
        #item

        #visibility trait #tracked_name: Send + Sync {
            type Error;
            #(#tracked_methods)*
        }

        impl<T> #tracked_name for T
        where
            T: #raw_name + Send + Sync + ?Sized,
        {
            type Error = <T as #raw_name>::Error;
            #(#implementations)*
        }
    })
}
