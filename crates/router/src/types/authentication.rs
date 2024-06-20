pub use hyperswitch_domain_models::{
    router_request_types::authentication::{
        AcquirerDetails, AuthNFlowType, ChallengeParams, ConnectorAuthenticationRequestData,
        ConnectorPostAuthenticationRequestData, PreAuthNRequestData, PreAuthenticationData,
    },
    router_response_types::AuthenticationResponseData,
};

use super::{api, RouterData};
use crate::services;

pub type PreAuthNRouterData =
    RouterData<api::PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>;

pub type PreAuthNVersionCallRouterData =
    RouterData<api::PreAuthenticationVersionCall, PreAuthNRequestData, AuthenticationResponseData>;

pub type ConnectorAuthenticationRouterData =
    RouterData<api::Authentication, ConnectorAuthenticationRequestData, AuthenticationResponseData>;

pub type ConnectorPostAuthenticationRouterData = RouterData<
    api::PostAuthentication,
    ConnectorPostAuthenticationRequestData,
    AuthenticationResponseData,
>;

pub type ConnectorAuthenticationType = dyn services::ConnectorIntegration<
    api::Authentication,
    ConnectorAuthenticationRequestData,
    AuthenticationResponseData,
>;

pub type ConnectorPostAuthenticationType = dyn services::ConnectorIntegration<
    api::PostAuthentication,
    ConnectorPostAuthenticationRequestData,
    AuthenticationResponseData,
>;

pub type ConnectorPreAuthenticationType = dyn services::ConnectorIntegration<
    api::PreAuthentication,
    PreAuthNRequestData,
    AuthenticationResponseData,
>;

pub type ConnectorPreAuthenticationVersionCallType = dyn services::ConnectorIntegration<
    api::PreAuthenticationVersionCall,
    PreAuthNRequestData,
    AuthenticationResponseData,
>;
