use hyperswitch_domain_models::{
    router_data_v2::ExternalAuthenticationFlowData,
    router_flow_types::authentication::{
        Authentication, PostAuthentication, PreAuthentication, PreAuthenticationVersionCall,
    },
    router_request_types::authentication::{
        ConnectorAuthenticationRequestData, ConnectorPostAuthenticationRequestData,
        PreAuthNRequestData,
    },
    router_response_types::AuthenticationResponseData,
};

use crate::api::ConnectorIntegrationV2;

/// trait ConnectorAuthenticationV2
pub trait ConnectorAuthenticationV2:
    ConnectorIntegrationV2<
    Authentication,
    ExternalAuthenticationFlowData,
    ConnectorAuthenticationRequestData,
    AuthenticationResponseData,
>
{
}

/// trait ConnectorPreAuthenticationV2
pub trait ConnectorPreAuthenticationV2:
    ConnectorIntegrationV2<
    PreAuthentication,
    ExternalAuthenticationFlowData,
    PreAuthNRequestData,
    AuthenticationResponseData,
>
{
}

/// trait ConnectorPreAuthenticationVersionCallV2
pub trait ConnectorPreAuthenticationVersionCallV2:
    ConnectorIntegrationV2<
    PreAuthenticationVersionCall,
    ExternalAuthenticationFlowData,
    PreAuthNRequestData,
    AuthenticationResponseData,
>
{
}

/// trait ConnectorPostAuthenticationV2
pub trait ConnectorPostAuthenticationV2:
    ConnectorIntegrationV2<
    PostAuthentication,
    ExternalAuthenticationFlowData,
    ConnectorPostAuthenticationRequestData,
    AuthenticationResponseData,
>
{
}

/// trait ExternalAuthenticationV2
pub trait ExternalAuthenticationV2:
    super::ConnectorCommon
    + ConnectorAuthenticationV2
    + ConnectorPreAuthenticationV2
    + ConnectorPreAuthenticationVersionCallV2
    + ConnectorPostAuthenticationV2
{
}
