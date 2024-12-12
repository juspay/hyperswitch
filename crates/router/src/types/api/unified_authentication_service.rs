use hyperswitch_domain_models::{
    router_data::RouterData,
    router_request_types::unified_authentication_service::{
        UasAuthenticationResponseData, UasPostAuthenticationRequestData,
        UasPreAuthenticationRequestData,
    },
};

pub use super::unified_authentication_service_v2::{
    UasPostAuthenticationV2, UasPreAuthenticationV2, UnifiedAuthenticationServiceV2,
};
use crate::services;

#[derive(Debug, Clone)]
pub struct PreAuthenticate;

pub trait UnifiedAuthenticationService:
    super::ConnectorCommon + UasPreAuthentication + UasPostAuthentication
{
}

#[derive(Debug, Clone)]
pub struct PostAuthenticate;

pub trait UasPreAuthentication:
    services::ConnectorIntegration<
    PreAuthenticate,
    UasPreAuthenticationRequestData,
    UasAuthenticationResponseData,
>
{
}

pub trait UasPostAuthentication:
    services::ConnectorIntegration<
    PostAuthenticate,
    UasPostAuthenticationRequestData,
    UasAuthenticationResponseData,
>
{
}

pub type UasPostAuthenticationRouterData =
    RouterData<PostAuthenticate, UasPostAuthenticationRequestData, UasAuthenticationResponseData>;

pub type UasPostAuthenticationType = dyn services::ConnectorIntegration<
    PostAuthenticate,
    UasPostAuthenticationRequestData,
    UasAuthenticationResponseData,
>;

pub type UasPreAuthenticationRouterData =
    RouterData<PreAuthenticate, UasPreAuthenticationRequestData, UasAuthenticationResponseData>;

pub type UasPreAuthenticationType = dyn services::ConnectorIntegration<
    PreAuthenticate,
    UasPreAuthenticationRequestData,
    UasAuthenticationResponseData,
>;
