use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{self, parse_macro_input, DeriveInput, Lit, Meta, MetaNameValue, NestedMeta};

#[derive(Debug, Clone, Copy)]
enum Derives {
    Sync,
    Cancel,
    Capture,
    Authorize,
    Authorizedata,
    Syncdata,
    Canceldata,
    Capturedata,
    VerifyData,
    Start,
    Verify,
    Session,
    SessionData,
}

impl From<String> for Derives {
    fn from(s: String) -> Self {
        match s.as_str() {
            "sync" => Self::Sync,
            "cancel" => Self::Cancel,
            "syncdata" => Self::Syncdata,
            "authorize" => Self::Authorize,
            "authorizedata" => Self::Authorizedata,
            "canceldata" => Self::Canceldata,
            "capture" => Self::Capture,
            "capturedata" => Self::Capturedata,
            "start" => Self::Start,
            "verify" => Self::Verify,
            "verifydata" => Self::VerifyData,
            "session" => Self::Session,
            "sessiondata" => Self::SessionData,
            _ => Self::Authorize,
        }
    }
}

impl Derives {
    fn to_operation(
        self,
        fns: impl Iterator<Item = TokenStream> + Clone,
        struct_name: &syn::Ident,
    ) -> TokenStream {
        let req_type = Conversion::get_req_type(self);
        quote! {
            #[automatically_derived]
            impl<F:Send+Clone> Operation<F,#req_type> for #struct_name {
                #(#fns)*
            }
        }
    }

    fn to_ref_operation(
        self,
        ref_fns: impl Iterator<Item = TokenStream> + Clone,
        struct_name: &syn::Ident,
    ) -> TokenStream {
        let req_type = Conversion::get_req_type(self);
        quote! {
            #[automatically_derived]
            impl<F:Send+Clone> Operation<F,#req_type> for &#struct_name {
                #(#ref_fns)*
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
enum Conversion {
    ValidateRequest,
    GetTracker,
    Domain,
    UpdateTracker,
    PostUpdateTracker,
    All,
}

impl From<String> for Conversion {
    fn from(s: String) -> Self {
        match s.as_str() {
            "validate_request" => Self::ValidateRequest,
            "get_tracker" => Self::GetTracker,
            "domain" => Self::Domain,
            "update_tracker" => Self::UpdateTracker,
            "post_tracker" => Self::PostUpdateTracker,
            "all" => Self::All,
            #[allow(clippy::panic)] // FIXME: Use `compile_error!()` instead
            _ => panic!("Invalid conversion identifier {}", s),
        }
    }
}

impl Conversion {
    fn get_req_type(ident: Derives) -> syn::Ident {
        match ident {
            Derives::Authorize => syn::Ident::new("PaymentsRequest", Span::call_site()),
            Derives::Authorizedata => syn::Ident::new("PaymentsAuthorizeData", Span::call_site()),
            Derives::Sync => syn::Ident::new("PaymentsRetrieveRequest", Span::call_site()),
            Derives::Syncdata => syn::Ident::new("PaymentsSyncData", Span::call_site()),
            Derives::Cancel => syn::Ident::new("PaymentsCancelRequest", Span::call_site()),
            Derives::Canceldata => syn::Ident::new("PaymentsCancelData", Span::call_site()),
            Derives::Capture => syn::Ident::new("PaymentsCaptureRequest", Span::call_site()),
            Derives::Capturedata => syn::Ident::new("PaymentsCaptureData", Span::call_site()),
            Derives::Start => syn::Ident::new("PaymentsStartRequest", Span::call_site()),
            Derives::Verify => syn::Ident::new("VerifyRequest", Span::call_site()),
            Derives::VerifyData => syn::Ident::new("VerifyRequestData", Span::call_site()),
            Derives::Session => syn::Ident::new("PaymentsSessionRequest", Span::call_site()),
            Derives::SessionData => syn::Ident::new("PaymentsSessionData", Span::call_site()),
        }
    }

    fn to_function(&self, ident: Derives) -> TokenStream {
        let req_type = Self::get_req_type(ident);
        match self {
            Self::ValidateRequest => quote! {
                fn to_validate_request(&self) -> RouterResult<&(dyn ValidateRequest<F,#req_type> + Send + Sync)> {
                    Ok(self)
                }
            },
            Self::GetTracker => quote! {
                fn to_get_tracker(&self) -> RouterResult<&(dyn GetTracker<F,PaymentData<F>,#req_type> + Send + Sync)> {
                    Ok(self)
                }
            },
            Self::Domain => quote! {
                fn to_domain(&self) -> RouterResult<&dyn Domain<F,#req_type>> {
                    Ok(self)
                }
            },
            Self::UpdateTracker => quote! {
                fn to_update_tracker(&self) -> RouterResult<&(dyn UpdateTracker<F,PaymentData<F>,#req_type> + Send + Sync)> {
                    Ok(self)
                }
            },
            Self::PostUpdateTracker => quote! {
                fn to_post_update_tracker(&self) -> RouterResult<&(dyn PostUpdateTracker<F, PaymentData<F>, #req_type> + Send + Sync)> {
                    Ok(self)
                }
            },
            Self::All => {
                let validate_request = Self::ValidateRequest.to_function(ident);
                let get_tracker = Self::GetTracker.to_function(ident);
                let domain = Self::Domain.to_function(ident);
                let update_tracker = Self::UpdateTracker.to_function(ident);

                quote! {
                    #validate_request
                    #get_tracker
                    #domain
                    #update_tracker
                }
            }
        }
    }

    fn to_ref_function(&self, ident: Derives) -> TokenStream {
        let req_type = Self::get_req_type(ident);
        match self {
            Self::ValidateRequest => quote! {
                fn to_validate_request(&self) -> RouterResult<&(dyn ValidateRequest<F,#req_type> + Send + Sync)> {
                    Ok(*self)
                }
            },
            Self::GetTracker => quote! {
                fn to_get_tracker(&self) -> RouterResult<&(dyn GetTracker<F,PaymentData<F>,#req_type> + Send + Sync)> {
                    Ok(*self)
                }
            },
            Self::Domain => quote! {
                fn to_domain(&self) -> RouterResult<&(dyn Domain<F,#req_type>)> {
                    Ok(*self)
                }
            },
            Self::UpdateTracker => quote! {
                fn to_update_tracker(&self) -> RouterResult<&(dyn UpdateTracker<F,PaymentData<F>,#req_type> + Send + Sync)> {
                    Ok(*self)
                }
            },
            Self::PostUpdateTracker => quote! {
                fn to_post_update_tracker(&self) -> RouterResult<&(dyn PostUpdateTracker<F, PaymentData<F>, #req_type> + Send + Sync)> {
                    Ok(*self)
                }
            },
            Self::All => {
                let validate_request = Self::ValidateRequest.to_ref_function(ident);
                let get_tracker = Self::GetTracker.to_ref_function(ident);
                let domain = Self::Domain.to_ref_function(ident);
                let update_tracker = Self::UpdateTracker.to_ref_function(ident);

                quote! {
                    #validate_request
                    #get_tracker
                    #domain
                    #update_tracker
                }
            }
        }
    }
}

fn find_operation_attr(a: &[syn::Attribute]) -> syn::Attribute {
    #[allow(clippy::expect_used)] // FIXME: Use `compile_error!()` instead
    a.iter()
        .find(|a| {
            a.path
                .get_ident()
                .map(|ident| *ident == "operation")
                .unwrap_or(false)
        })
        .expect("Missing 'operation' attribute with conversions")
        .clone()
}

fn find_value(v: NestedMeta) -> Option<(String, Vec<String>)> {
    match v {
        NestedMeta::Meta(Meta::NameValue(MetaNameValue {
            ref path,
            eq_token: _,
            lit: Lit::Str(ref litstr),
        })) => {
            let key = path.get_ident()?.to_string();
            Some((
                key,
                litstr.value().split(',').map(ToString::to_string).collect(),
            ))
        }
        _ => None,
    }
}

// FIXME: Propagate errors in a better manner instead of `expect()`, maybe use `compile_error!()`
#[allow(clippy::unwrap_used, clippy::expect_used)]
fn find_properties(attr: &syn::Attribute) -> Option<HashMap<String, Vec<String>>> {
    let meta = attr.parse_meta();
    match meta {
        Ok(syn::Meta::List(syn::MetaList {
            ref path,
            paren_token: _,
            mut nested,
        })) => {
            path.get_ident().map(|i| i == "operation")?;
            let tracker = nested.pop().unwrap().into_value();
            let operation = nested.pop().unwrap().into_value();
            let o = find_value(tracker).expect("Invalid format of attributes");
            let t = find_value(operation).expect("Invalid format of attributes");
            Some(HashMap::from_iter([t, o]))
        }
        _ => None,
    }
}

// FIXME: Propagate errors in a better manner instead of `expect()`, maybe use `compile_error!()`
#[allow(clippy::expect_used)]
pub fn operation_derive_inner(token: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(token as DeriveInput);
    let struct_name = &input.ident;
    let op = find_operation_attr(&input.attrs);
    let prop = find_properties(&op);
    let ops = prop.as_ref().expect("Invalid Properties")["ops"].clone();
    let flow = prop.as_ref().expect("Invalid Properties")["flow"].clone();
    let trait_derive = flow.iter().map(|derive| {
        let derive: Derives = derive.to_owned().into();
        let fns = ops.iter().map(|t| {
            let con: Conversion = t.to_owned().into();
            con.to_function(derive)
        });
        derive.to_operation(fns, struct_name)
    });
    let ref_trait_derive = flow.iter().map(|derive| {
        let derive: Derives = derive.to_owned().into();
        let fns = ops.iter().map(|t| {
            let con: Conversion = t.to_owned().into();
            con.to_ref_function(derive)
        });
        derive.to_ref_operation(fns, struct_name)
    });
    let trait_derive = quote! {
            #(#ref_trait_derive)* #(#trait_derive)*
    };
    let output = quote! {
        const _: () = {
                use crate::core::errors::RouterResult;
                use crate::core::payments::operations::{
                    ValidateRequest,
                    PostUpdateTracker,
                    GetTracker,
                    UpdateTracker,
                    PaymentData
                };
                use crate::types::{
                    VerifyRequestData,
                    PaymentsSyncData,
                    PaymentsCaptureData,
                    PaymentsCancelData,
                    PaymentsAuthorizeData,
                    PaymentsSessionData,

                    api::{
                        PaymentsCaptureRequest,
                        PaymentsCancelRequest,
                        PaymentsRetrieveRequest,
                        PaymentsRequest,
                        PaymentsStartRequest,
                        PaymentsSessionRequest,
                        VerifyRequest
                    }
                };
                #trait_derive
            };
    };
    proc_macro::TokenStream::from(output)
}
