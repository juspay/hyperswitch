//! PaymentGateway implementation for api::PSync flow
//!
//! This module implements the PaymentGateway trait for the PSync (Payment Sync) flow,
//! handling payment status synchronization via the payment_get GRPC endpoint.

use async_trait::async_trait;
use std::str::FromStr;
use common_enums::{connector_enums::Connector, CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, request::Request};
use error_stack::ResultExt;
use external_services::grpc_client::unified_connector_service::UnifiedConnectorServiceClient;
use hyperswitch_domain_models::{
    router_flow_types as domain,
    router_data::{ErrorResponse, RouterData},
};
use hyperswitch_interfaces::{
    api::{self, gateway as payment_gateway},
    api_client::ApiClientWrapper,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
    unified_connector_service::{
        UcsContext, UcsFlowExecutor, UcsGrpcExecutor, UcsRequestTransformer, UcsResponseHandler,
    },
};
use masking::Secret;
use unified_connector_service_client::payments as payments_grpc;

use crate::core::payments::gateway::RouterGatewayContext;

use super::{
    helpers::prepare_ucs_infrastructure,
    ucs_context::RouterUcsContext,
    ucs_execution_context::RouterUcsExecutionContext,
    ucs_executors::ucs_executor,
};
use hyperswitch_interfaces::unified_connector_service::handle_unified_connector_service_response_for_payment_get;
use crate::{
    define_ucs_executor,
    routes::SessionState,
    types::{self, transformers::ForeignTryFrom},
};

// =============================================================================
// PSyncUcsExecutor - UCS Flow Executor for Payment Sync
// =============================================================================

define_ucs_executor! {
    executor: PSyncUcsExecutor,
    flow: domain::PSync,
    request_data: types::PaymentsSyncData,
    response_data: types::PaymentsResponseData,
    grpc_request: payments_grpc::PaymentServiceGetRequest,
    grpc_response: payments_grpc::PaymentServiceGetResponse,
    grpc_method: payment_get,
    response_handler: handle_unified_connector_service_response_for_payment_get,
}

// =============================================================================
// PaymentGateway Implementation for domain::PSync
// =============================================================================

/// Implementation of PaymentGateway for api::PSync flow
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::PSync,
        types::PaymentsSyncData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::PSync
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::PSync,
        types::PaymentsSyncData,
        types::PaymentsResponseData,>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::PSync,
            RCD,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<domain::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<domain::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        ConnectorError,
    > {
        // Check if UCS PSync is disabled for this connector
        let connector_enum = Connector::from_str(&router_data.connector)
            .change_context(ConnectorError::InvalidConnectorName)?;

        if is_psync_disabled(state, &connector_enum) {
            return Err(ConnectorError::NotImplemented(format!(
                "UCS PSync disabled for connector: {}",
                router_data.connector
            ))
            .into());
        }

        // Create execution context
        let execution_context = RouterUcsExecutionContext::new(
            &context.merchant_context,
            &context.header_payload,
            context.lineage_ids,
            &context.merchant_connector_account,
            context.execution_mode,
        );

        // Execute payment_get GRPC call using trait-based executor
        PSyncUcsExecutor::execute_ucs_flow(
            state,
            router_data,
            execution_context,
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
        types::PaymentsSyncData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::PSync
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::PSync,
        types::PaymentsSyncData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => {
                Box::new(payment_gateway::DirectGateway)
            }
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => {
                Box::new(domain::PSync)
            }
        }
    }
}



/// Check if UCS PSync is disabled for a connector
fn is_psync_disabled(state: &SessionState, connector: &Connector) -> bool {
    state
        .conf
        .grpc_client
        .unified_connector_service
        .as_ref()
        .is_some_and(|config| config.ucs_psync_disabled_connectors.contains(connector))
}