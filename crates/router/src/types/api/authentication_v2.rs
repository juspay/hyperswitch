pub use hyperswitch_domain_models::router_request_types::authentication::MessageCategory;

use super::authentication::{
    Authentication, PostAuthentication, PreAuthentication, PreAuthenticationVersionCall,
};
use crate::{services, types};

pub trait ConnectorAuthenticationV2:
    services::ConnectorIntegrationV2<
    Authentication,
    types::ExternalAuthenticationFlowData,
    types::authentication::ConnectorAuthenticationRequestData,
    types::authentication::AuthenticationResponseData,
>
{
}

pub trait ConnectorPreAuthenticationV2:
    services::ConnectorIntegrationV2<
    PreAuthentication,
    types::ExternalAuthenticationFlowData,
    types::authentication::PreAuthNRequestData,
    types::authentication::AuthenticationResponseData,
>
{
}

pub trait ConnectorPreAuthenticationVersionCallV2:
    services::ConnectorIntegrationV2<
    PreAuthenticationVersionCall,
    types::ExternalAuthenticationFlowData,
    types::authentication::PreAuthNRequestData,
    types::authentication::AuthenticationResponseData,
>
{
}

pub trait ConnectorPostAuthenticationV2:
    services::ConnectorIntegrationV2<
    PostAuthentication,
    types::ExternalAuthenticationFlowData,
    types::authentication::ConnectorPostAuthenticationRequestData,
    types::authentication::AuthenticationResponseData,
>
{
}

pub trait ExternalAuthenticationV2:
    super::ConnectorCommon
    + ConnectorAuthenticationV2
    + ConnectorPreAuthenticationV2
    + ConnectorPreAuthenticationVersionCallV2
    + ConnectorPostAuthenticationV2
{
}
