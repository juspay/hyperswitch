use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, request::Request};
use hyperswitch_domain_models::{
    router_data::RouterData, router_flow_types::authentication as auth_flows,
    router_request_types::authentication as auth_request,
    router_response_types::AuthenticationResponseData,
};
use hyperswitch_interfaces::{
    api::gateway as payment_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};

use crate::{
    core::{authentication::ucs, payments::gateway::context::RouterGatewayContext},
    routes::SessionState,
};

macro_rules! impl_external_authentication_gateway {
    ($flow:ty, $request:ty, $ucs_call:path) => {
        #[async_trait]
        impl<RCD>
            payment_gateway::PaymentGateway<
                SessionState,
                RCD,
                Self,
                $request,
                AuthenticationResponseData,
                RouterGatewayContext,
            > for $flow
        where
            RCD: Clone
                + Send
                + Sync
                + 'static
                + RouterDataConversion<Self, $request, AuthenticationResponseData>,
        {
            async fn execute(
                self: Box<Self>,
                state: &SessionState,
                _connector_integration: BoxedConnectorIntegrationInterface<
                    Self,
                    RCD,
                    $request,
                    AuthenticationResponseData,
                >,
                router_data: &RouterData<Self, $request, AuthenticationResponseData>,
                _call_connector_action: CallConnectorAction,
                _connector_request: Option<Request>,
                _return_raw_connector_response: Option<bool>,
                context: RouterGatewayContext,
            ) -> CustomResult<RouterData<Self, $request, AuthenticationResponseData>, ConnectorError>
            {
                Box::pin($ucs_call(router_data, state, &context)).await
            }
        }

        impl<RCD>
            payment_gateway::FlowGateway<
                SessionState,
                RCD,
                $request,
                AuthenticationResponseData,
                RouterGatewayContext,
            > for $flow
        where
            RCD: Clone
                + Send
                + Sync
                + 'static
                + RouterDataConversion<Self, $request, AuthenticationResponseData>,
        {
            fn get_gateway(
                execution_path: ExecutionPath,
            ) -> Box<
                dyn payment_gateway::PaymentGateway<
                    SessionState,
                    RCD,
                    Self,
                    $request,
                    AuthenticationResponseData,
                    RouterGatewayContext,
                >,
            > {
                match execution_path {
                    ExecutionPath::Direct => Box::new(payment_gateway::DirectGateway),
                    ExecutionPath::UnifiedConnectorService
                    | ExecutionPath::ShadowUnifiedConnectorService => Box::new(Self),
                }
            }
        }
    };
}

impl_external_authentication_gateway!(
    auth_flows::PreAuthentication,
    auth_request::PreAuthNRequestData,
    ucs::call_unified_connector_service_pre_authentication
);
impl_external_authentication_gateway!(
    auth_flows::Authentication,
    auth_request::ConnectorAuthenticationRequestData,
    ucs::call_unified_connector_service_authentication
);
impl_external_authentication_gateway!(
    auth_flows::PostAuthentication,
    auth_request::ConnectorPostAuthenticationRequestData,
    ucs::call_unified_connector_service_post_authentication
);
