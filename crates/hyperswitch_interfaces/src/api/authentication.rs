use hyperswitch_domain_models::{
    router_flow_types::authentication::{
        Authentication, PostAuthentication, PreAuthentication, PreAuthenticationVersionCall,
    },
    router_request_types::authentication::{
        ConnectorAuthenticationRequestData, ConnectorPostAuthenticationRequestData,
        PreAuthNRequestData,
    },
    router_response_types::AuthenticationResponseData,
};

use crate::api::ConnectorIntegration;

/// trait ConnectorAuthentication
pub trait ConnectorAuthentication:
    ConnectorIntegration<Authentication, ConnectorAuthenticationRequestData, AuthenticationResponseData>
{
}

/// trait ConnectorPreAuthentication
pub trait ConnectorPreAuthentication:
    ConnectorIntegration<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>
{
}

/// trait ConnectorPreAuthenticationVersionCall
pub trait ConnectorPreAuthenticationVersionCall:
    ConnectorIntegration<PreAuthenticationVersionCall, PreAuthNRequestData, AuthenticationResponseData>
{
}

/// trait ConnectorPostAuthentication
pub trait ConnectorPostAuthentication:
    ConnectorIntegration<
    PostAuthentication,
    ConnectorPostAuthenticationRequestData,
    AuthenticationResponseData,
>
{
}

/// trait ExternalAuthentication
pub trait ExternalAuthentication:
    super::ConnectorCommon
    + ConnectorAuthentication
    + ConnectorPreAuthentication
    + ConnectorPreAuthenticationVersionCall
    + ConnectorPostAuthentication
{
}
