use std::str::FromStr;

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, request::Request};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::RouterData, router_flow_types as domain, router_request_types,
};
use hyperswitch_interfaces::{
    api::gateway as payment_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};

use crate::{
    core::payments::{flows::complete_authorize_flow, gateway::context::RouterGatewayContext},
    routes::SessionState,
    types,
};

// =============================================================================
// PaymentGateway Implementation for domain::Authenticate
// =============================================================================

/// Implementation of PaymentGateway for api::Authenticate flow
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        Self,
        types::PaymentsAuthenticateData,
        types::PaymentsResponseData,
        RouterGatewayContext,
        Option<router_request_types::UcsAuthenticationData>,
    > for domain::Authenticate
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::PaymentsAuthenticateData, types::PaymentsResponseData>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            Self,
            RCD,
            types::PaymentsAuthenticateData,
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<
            Self,
            types::PaymentsAuthenticateData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        (
            RouterData<Self, types::PaymentsAuthenticateData, types::PaymentsResponseData>,
            Option<router_request_types::UcsAuthenticationData>,
        ),
        ConnectorError,
    > {
        let merchant_connector_account = context.merchant_connector_account;
        let platform = context.platform;
        let lineage_ids = context.lineage_ids;
        let header_payload = context.header_payload;
        let unified_connector_service_execution_mode = context.execution_mode;
        let merchant_order_reference_id = header_payload.x_reference_id.clone();
        let connector_enum =
            common_enums::connector_enums::Connector::from_str(&router_data.connector)
                .change_context(ConnectorError::InvalidConnectorName)
                .attach_printable("Invalid connector name")?;
        complete_authorize_flow::call_unified_connector_service_authenticate(
            router_data,
            state,
            &header_payload,
            lineage_ids,
            merchant_connector_account,
            &platform,
            connector_enum,
            unified_connector_service_execution_mode,
            merchant_order_reference_id,
        )
        .await
    }
}

/// Implementation of FlowGateway for api::PSync
///
/// This allows the flow to provide its specific gateway based on execution path
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PaymentsAuthenticateData,
        types::PaymentsResponseData,
        RouterGatewayContext,
        Option<router_request_types::UcsAuthenticationData>,
    > for domain::Authenticate
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::PaymentsAuthenticateData, types::PaymentsResponseData>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsAuthenticateData,
            types::PaymentsResponseData,
            RouterGatewayContext,
            Option<router_request_types::UcsAuthenticationData>,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => Box::new(payment_gateway::DirectGateway),
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => Box::new(Self),
        }
    }
}
