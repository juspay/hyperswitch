use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{self, spanned::Spanned, DeriveInput, Lit, Meta, MetaNameValue, NestedMeta};

use crate::macros::helpers;

#[derive(Debug, Clone, Copy)]
enum Derives {
    Sync,
    Cancel,
    Reject,
    Capture,
    Approvedata,
    Authorize,
    Authorizedata,
    Syncdata,
    Canceldata,
    Capturedata,
    CompleteAuthorizeData,
    Rejectdata,
    SetupMandateData,
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
            "reject" => Self::Reject,
            "syncdata" => Self::Syncdata,
            "authorize" => Self::Authorize,
            "approvedata" => Self::Approvedata,
            "authorizedata" => Self::Authorizedata,
            "canceldata" => Self::Canceldata,
            "capture" => Self::Capture,
            "capturedata" => Self::Capturedata,
            "completeauthorizedata" => Self::CompleteAuthorizeData,
            "rejectdata" => Self::Rejectdata,
            "start" => Self::Start,
            "verify" => Self::Verify,
            "setupmandatedata" => Self::SetupMandateData,
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
    Invalid(String),
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
            s => Self::Invalid(s.to_string()),
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
            Derives::Approvedata => syn::Ident::new("PaymentsApproveData", Span::call_site()),
            Derives::Reject => syn::Ident::new("PaymentsRejectRequest", Span::call_site()),
            Derives::Rejectdata => syn::Ident::new("PaymentsRejectData", Span::call_site()),
            Derives::Capture => syn::Ident::new("PaymentsCaptureRequest", Span::call_site()),
            Derives::Capturedata => syn::Ident::new("PaymentsCaptureData", Span::call_site()),
            Derives::CompleteAuthorizeData => {
                syn::Ident::new("CompleteAuthorizeData", Span::call_site())
            }
            Derives::Start => syn::Ident::new("PaymentsStartRequest", Span::call_site()),
            Derives::Verify => syn::Ident::new("VerifyRequest", Span::call_site()),
            Derives::SetupMandateData => {
                syn::Ident::new("SetupMandateRequestData", Span::call_site())
            }
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
            Self::Invalid(s) => {
                helpers::syn_error(Span::call_site(), &format!("Invalid identifier {s}"))
                    .to_compile_error()
            }
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
            Self::Invalid(s) => {
                helpers::syn_error(Span::call_site(), &format!("Invalid identifier {s}"))
                    .to_compile_error()
            }
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

fn find_operation_attr(a: &[syn::Attribute]) -> syn::Result<syn::Attribute> {
    a.iter()
        .find(|a| {
            a.path
                .get_ident()
                .map(|ident| *ident == "operation")
                .unwrap_or(false)
        })
        .cloned()
        .ok_or_else(|| {
            helpers::syn_error(
                Span::call_site(),
                "Cannot find attribute 'operation' in the macro",
            )
        })
}

fn find_value(v: &NestedMeta) -> Option<(String, Vec<String>)> {
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

fn find_properties(attr: &syn::Attribute) -> syn::Result<HashMap<String, Vec<String>>> {
    let meta = attr.parse_meta();
    match meta {
        Ok(syn::Meta::List(syn::MetaList {
            ref path,
            paren_token: _,
            nested,
        })) => {
            path.get_ident().map(|i| i == "operation").ok_or_else(|| {
                helpers::syn_error(path.span(), "Attribute 'operation' was not found")
            })?;
            Ok(HashMap::from_iter(nested.iter().filter_map(find_value)))
        }
        _ => Err(helpers::syn_error(
            attr.span(),
            "No attributes were found. Expected format is ops=..,flow=..",
        )),
    }
}

pub fn operation_derive_inner(input: DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    let struct_name = &input.ident;
    let op = find_operation_attr(&input.attrs)?;
    let prop = find_properties(&op)?;
    let ops = prop.get("ops").ok_or_else(|| {
        helpers::syn_error(
            op.span(),
            "Invalid properties. Property 'ops' was not found",
        )
    })?;
    let flow = prop.get("flow").ok_or_else(|| {
        helpers::syn_error(
            op.span(),
            "Invalid properties. Property 'flow' was not found",
        )
    })?;
    let current_crate = syn::Ident::new(
        &prop
            .get("crate")
            .map(|v| v.join(""))
            .unwrap_or_else(|| String::from("crate")),
        Span::call_site(),
    );

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
                use #current_crate::core::errors::RouterResult;
                use #current_crate::core::payments::{PaymentData,operations::{
                    ValidateRequest,
                    PostUpdateTracker,
                    GetTracker,
                    UpdateTracker,
                }};
                use #current_crate::types::{
                    SetupMandateRequestData,
                    PaymentsSyncData,
                    PaymentsCaptureData,
                    PaymentsCancelData,
                    PaymentsApproveData,
                    PaymentsRejectData,
                    PaymentsAuthorizeData,
                    PaymentsSessionData,
                    CompleteAuthorizeData,

                    api::{
                        PaymentsCaptureRequest,
                        PaymentsCancelRequest,
                        PaymentsApproveRequest,
                        PaymentsRejectRequest,
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
    Ok(proc_macro::TokenStream::from(output))
}
