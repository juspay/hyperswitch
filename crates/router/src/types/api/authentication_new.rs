pub use hyperswitch_domain_models::router_request_types::authentication::MessageCategory;

use super::authentication::{
    Authentication, PostAuthentication, PreAuthentication, PreAuthenticationVersionCall,
};
use crate::{services, types};

pub trait ConnectorAuthenticationNew:
    services::ConnectorIntegrationNew<
    Authentication,
    types::ExternalAuthenticationFlowData,
    types::authentication::ConnectorAuthenticationRequestData,
    types::authentication::AuthenticationResponseData,
>
{
}

pub trait ConnectorPreAuthenticationNew:
    services::ConnectorIntegrationNew<
    PreAuthentication,
    types::ExternalAuthenticationFlowData,
    types::authentication::PreAuthNRequestData,
    types::authentication::AuthenticationResponseData,
>
{
}

pub trait ConnectorPreAuthenticationVersionCallNew:
    services::ConnectorIntegrationNew<
    PreAuthenticationVersionCall,
    types::ExternalAuthenticationFlowData,
    types::authentication::PreAuthNRequestData,
    types::authentication::AuthenticationResponseData,
>
{
}

pub trait ConnectorPostAuthenticationNew:
    services::ConnectorIntegrationNew<
    PostAuthentication,
    types::ExternalAuthenticationFlowData,
    types::authentication::ConnectorPostAuthenticationRequestData,
    types::authentication::AuthenticationResponseData,
>
{
}

pub trait ExternalAuthenticationNew:
    super::ConnectorCommon
    + ConnectorAuthenticationNew
    + ConnectorPreAuthenticationNew
    + ConnectorPreAuthenticationVersionCallNew
    + ConnectorPostAuthenticationNew
{
}
