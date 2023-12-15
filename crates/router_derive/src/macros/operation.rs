use std::str::FromStr;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use strum::IntoEnumIterator;
use syn::{self, parse::Parse, DeriveInput};

use crate::macros::helpers::{self};

#[derive(Debug, Clone, Copy, strum::EnumString, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Derives {
    Sync,
    Cancel,
    Reject,
    Capture,
    ApproveData,
    Authorize,
    AuthorizeData,
    SyncData,
    CancelData,
    CaptureData,
    CompleteAuthorizeData,
    RejectData,
    SetupMandateData,
    Start,
    Verify,
    Session,
    SessionData,
    IncrementalAuthorization,
    IncrementalAuthorizationData,
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
            impl<F:Send+Clone,Ctx: PaymentMethodRetrieve,> Operation<F,#req_type,Ctx> for #struct_name {
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
            impl<F:Send+Clone,Ctx: PaymentMethodRetrieve,> Operation<F,#req_type,Ctx> for &#struct_name {
                #(#ref_fns)*
            }
        }
    }
}

#[derive(Debug, Clone, strum::EnumString, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Conversion {
    ValidateRequest,
    GetTracker,
    Domain,
    UpdateTracker,
    PostUpdateTracker,
    All,
    Invalid(String),
}

impl Conversion {
    fn get_req_type(ident: Derives) -> syn::Ident {
        match ident {
            Derives::Authorize => syn::Ident::new("PaymentsRequest", Span::call_site()),
            Derives::AuthorizeData => syn::Ident::new("PaymentsAuthorizeData", Span::call_site()),
            Derives::Sync => syn::Ident::new("PaymentsRetrieveRequest", Span::call_site()),
            Derives::SyncData => syn::Ident::new("PaymentsSyncData", Span::call_site()),
            Derives::Cancel => syn::Ident::new("PaymentsCancelRequest", Span::call_site()),
            Derives::CancelData => syn::Ident::new("PaymentsCancelData", Span::call_site()),
            Derives::ApproveData => syn::Ident::new("PaymentsApproveData", Span::call_site()),
            Derives::Reject => syn::Ident::new("PaymentsRejectRequest", Span::call_site()),
            Derives::RejectData => syn::Ident::new("PaymentsRejectData", Span::call_site()),
            Derives::Capture => syn::Ident::new("PaymentsCaptureRequest", Span::call_site()),
            Derives::CaptureData => syn::Ident::new("PaymentsCaptureData", Span::call_site()),
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
            Derives::IncrementalAuthorization => {
                syn::Ident::new("PaymentsIncrementalAuthorizationRequest", Span::call_site())
            }
            Derives::IncrementalAuthorizationData => {
                syn::Ident::new("PaymentsIncrementalAuthorizationData", Span::call_site())
            }
        }
    }

    fn to_function(&self, ident: Derives) -> TokenStream {
        let req_type = Self::get_req_type(ident);
        match self {
            Self::ValidateRequest => quote! {
                fn to_validate_request(&self) -> RouterResult<&(dyn ValidateRequest<F,#req_type,Ctx> + Send + Sync)> {
                    Ok(self)
                }
            },
            Self::GetTracker => quote! {
                fn to_get_tracker(&self) -> RouterResult<&(dyn GetTracker<F,PaymentData<F>,#req_type,Ctx> + Send + Sync)> {
                    Ok(self)
                }
            },
            Self::Domain => quote! {
                fn to_domain(&self) -> RouterResult<&dyn Domain<F,#req_type,Ctx>> {
                    Ok(self)
                }
            },
            Self::UpdateTracker => quote! {
                fn to_update_tracker(&self) -> RouterResult<&(dyn UpdateTracker<F,PaymentData<F>,#req_type,Ctx> + Send + Sync)> {
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
                fn to_validate_request(&self) -> RouterResult<&(dyn ValidateRequest<F,#req_type,Ctx> + Send + Sync)> {
                    Ok(*self)
                }
            },
            Self::GetTracker => quote! {
                fn to_get_tracker(&self) -> RouterResult<&(dyn GetTracker<F,PaymentData<F>,#req_type,Ctx> + Send + Sync)> {
                    Ok(*self)
                }
            },
            Self::Domain => quote! {
                fn to_domain(&self) -> RouterResult<&(dyn Domain<F,#req_type,Ctx>)> {
                    Ok(*self)
                }
            },
            Self::UpdateTracker => quote! {
                fn to_update_tracker(&self) -> RouterResult<&(dyn UpdateTracker<F,PaymentData<F>,#req_type,Ctx> + Send + Sync)> {
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

mod operations_keyword {
    use syn::custom_keyword;

    custom_keyword!(operations);
    custom_keyword!(flow);
}

#[derive(Debug)]
pub enum OperationsEnumMeta {
    Operations {
        keyword: operations_keyword::operations,
        value: Vec<Conversion>,
    },
    Flow {
        keyword: operations_keyword::flow,
        value: Vec<Derives>,
    },
}

#[derive(Clone)]
pub struct OperationProperties {
    operations: Vec<Conversion>,
    flows: Vec<Derives>,
}

fn get_operation_properties(
    operation_enums: Vec<OperationsEnumMeta>,
) -> syn::Result<OperationProperties> {
    let mut operations = vec![];
    let mut flows = vec![];

    for operation in operation_enums {
        match operation {
            OperationsEnumMeta::Operations { value, .. } => {
                operations = value;
            }
            OperationsEnumMeta::Flow { value, .. } => {
                flows = value;
            }
        }
    }

    if operations.is_empty() {
        Err(syn::Error::new(
            Span::call_site(),
            "atleast one operation must be specitied",
        ))?;
    }

    if flows.is_empty() {
        Err(syn::Error::new(
            Span::call_site(),
            "atleast one flow must be specitied",
        ))?;
    }

    Ok(OperationProperties { operations, flows })
}

impl Parse for Derives {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let text = input.parse::<syn::LitStr>()?;
        let value = text.value();

        value.as_str().parse().map_err(|_| {
            syn::Error::new_spanned(
                &text,
                format!(
                    "Unexpected value for flow: `{value}`. Possible values are `{}`",
                    helpers::get_possible_values_for_enum::<Self>()
                ),
            )
        })
    }
}

impl Parse for Conversion {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let text = input.parse::<syn::LitStr>()?;
        let value = text.value();

        value.as_str().parse().map_err(|_| {
            syn::Error::new_spanned(
                &text,
                format!(
                    "Unexpected value for operation: `{value}`. Possible values are `{}`",
                    helpers::get_possible_values_for_enum::<Self>()
                ),
            )
        })
    }
}

fn parse_list_string<T>(list_string: String, keyword: &str) -> syn::Result<Vec<T>>
where
    T: FromStr + IntoEnumIterator + ToString,
{
    list_string
        .split(',')
        .map(str::trim)
        .map(T::from_str)
        .map(|result| {
            result.map_err(|_| {
                syn::Error::new(
                    Span::call_site(),
                    format!(
                        "Unexpected {keyword}, possible values are {}",
                        helpers::get_possible_values_for_enum::<T>()
                    ),
                )
            })
        })
        .collect()
}

fn get_conversions(input: syn::parse::ParseStream<'_>) -> syn::Result<Vec<Conversion>> {
    let lit_str_list = input.parse::<syn::LitStr>()?;
    parse_list_string(lit_str_list.value(), "operation")
}

fn get_derives(input: syn::parse::ParseStream<'_>) -> syn::Result<Vec<Derives>> {
    let lit_str_list = input.parse::<syn::LitStr>()?;
    parse_list_string(lit_str_list.value(), "flow")
}

impl Parse for OperationsEnumMeta {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(operations_keyword::operations) {
            let keyword = input.parse()?;
            input.parse::<syn::Token![=]>()?;
            let value = get_conversions(input)?;
            Ok(Self::Operations { keyword, value })
        } else if lookahead.peek(operations_keyword::flow) {
            let keyword = input.parse()?;
            input.parse::<syn::Token![=]>()?;
            let value = get_derives(input)?;
            Ok(Self::Flow { keyword, value })
        } else {
            Err(lookahead.error())
        }
    }
}

trait OperationsDeriveInputExt {
    /// Get all the error metadata associated with an enum.
    fn get_metadata(&self) -> syn::Result<Vec<OperationsEnumMeta>>;
}

impl OperationsDeriveInputExt for DeriveInput {
    fn get_metadata(&self) -> syn::Result<Vec<OperationsEnumMeta>> {
        helpers::get_metadata_inner("operation", &self.attrs)
    }
}

impl ToTokens for OperationsEnumMeta {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Operations { keyword, .. } => keyword.to_tokens(tokens),
            Self::Flow { keyword, .. } => keyword.to_tokens(tokens),
        }
    }
}

pub fn operation_derive_inner(input: DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    let struct_name = &input.ident;
    let operations_meta = input.get_metadata()?;
    let operation_properties = get_operation_properties(operations_meta)?;

    let current_crate = syn::Ident::new("crate", Span::call_site());

    let trait_derive = operation_properties
        .clone()
        .flows
        .into_iter()
        .map(|derive| {
            let fns = operation_properties
                .operations
                .iter()
                .map(|conversion| conversion.to_function(derive));
            derive.to_operation(fns, struct_name)
        })
        .collect::<Vec<_>>();

    let ref_trait_derive = operation_properties
        .flows
        .into_iter()
        .map(|derive| {
            let fns = operation_properties
                .operations
                .iter()
                .map(|conversion| conversion.to_ref_function(derive));
            derive.to_ref_operation(fns, struct_name)
        })
        .collect::<Vec<_>>();

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
                    PaymentsIncrementalAuthorizationData,

                    api::{
                        PaymentsCaptureRequest,
                        PaymentsCancelRequest,
                        PaymentsApproveRequest,
                        PaymentsRejectRequest,
                        PaymentsRetrieveRequest,
                        PaymentsRequest,
                        PaymentsStartRequest,
                        PaymentsSessionRequest,
                        VerifyRequest,
                        PaymentsIncrementalAuthorizationRequest
                    }
                };
                #trait_derive
            };
    };
    Ok(proc_macro::TokenStream::from(output))
}
