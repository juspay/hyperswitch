use hyperswitch_domain_models::{
    router_data_v2::UasFlowData,
    router_request_types::unified_authentication_service::{
        UasAuthenticationResponseData, UasPostAuthenticationRequestData,
        UasPreAuthenticationRequestData,
    },
};

use super::unified_authentication_service::{PostAuthenticate, PreAuthenticate};
use crate::services;

pub trait UnifiedAuthenticationServiceV2:
    super::ConnectorCommon + UasPreAuthenticationV2 + UasPostAuthenticationV2
{
}

pub trait UasPreAuthenticationV2:
    services::ConnectorIntegrationV2<
    PreAuthenticate,
    UasFlowData,
    UasPreAuthenticationRequestData,
    UasAuthenticationResponseData,
>
{
}

pub trait UasPostAuthenticationV2:
    services::ConnectorIntegrationV2<
    PostAuthenticate,
    UasFlowData,
    UasPostAuthenticationRequestData,
    UasAuthenticationResponseData,
>
{
}
