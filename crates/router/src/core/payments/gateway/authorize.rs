//! PaymentGateway implementation for api::Authorize flow
//!
//! This module implements the PaymentGateway trait for the Authorize flow,
//! handling both regular payments (payment_authorize) and mandate payments (payment_repeat).

// =============================================================================
// Imports
// =============================================================================

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, request::Request};
use error_stack::ResultExt;
use external_services::grpc_client::unified_connector_service::UnifiedConnectorServiceClient;
use hyperswitch_domain_models::{
    router_data::{ErrorResponse, RouterData},
    router_flow_types as domain,
};
use hyperswitch_interfaces::{
    api::gateway as payment_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
    unified_connector_service::{
        UcsContext, UcsExecutionContextProvider, UcsFlowExecutor, UcsGrpcExecutor,
        UcsRequestTransformer, UcsResponseHandler,
    },
};
use masking::Secret;
use unified_connector_service_client::payments as payments_grpc;

use super::{
    context::RouterGatewayContext,
    helpers::prepare_ucs_infrastructure,
    ucs_context::RouterUcsContext,
    ucs_execution_context::RouterUcsExecutionContext,
    ucs_executors::ucs_executor,
};
use crate::{
    core::unified_connector_service::{
        handle_unified_connector_service_response_for_payment_authorize,
        handle_unified_connector_service_response_for_payment_repeat,
    },
    define_ucs_executor,
    routes::SessionState,
    types::{self, transformers::ForeignTryFrom},
};

// =============================================================================
// AuthorizeUcsExecutor - UCS Flow Executor for Authorize
// =============================================================================

define_ucs_executor! {
    executor: AuthorizeUcsExecutor,
    flow: domain::Authorize,
    request_data: types::PaymentsAuthorizeData,
    response_data: types::PaymentsResponseData,
    grpc_request: payments_grpc::PaymentServiceAuthorizeRequest,
    grpc_response: payments_grpc::PaymentServiceAuthorizeResponse,
    grpc_method: payment_authorize,
    response_handler: handle_unified_connector_service_response_for_payment_authorize,
}

// =============================================================================
// RepeatUcsExecutor - UCS Flow Executor for Mandate Payments
// =============================================================================

define_ucs_executor! {
    executor: RepeatUcsExecutor,
    flow: domain::Authorize,
    request_data: types::PaymentsAuthorizeData,
    response_data: types::PaymentsResponseData,
    grpc_request: payments_grpc::PaymentServiceRepeatEverythingRequest,
    grpc_response: payments_grpc::PaymentServiceRepeatEverythingResponse,
    grpc_method: payment_repeat,
    response_handler: handle_unified_connector_service_response_for_payment_repeat,
}

// =============================================================================
// PaymentGateway Implementation for domain::Authorize - Using Macro
// =============================================================================

// =============================================================================
// PaymentGateway Implementation for domain::Authorize
// =============================================================================

#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::Authorize
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            domain::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::Authorize,
            RCD,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<
            domain::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<domain::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        ConnectorError,
    > {
        let execution_context = RouterUcsExecutionContext::new(
            &context.merchant_context,
            &context.header_payload,
            context.lineage_ids,
            &context.merchant_connector_account,
            context.execution_mode,
        );

        if router_data.request.mandate_id.is_some() {
            RepeatUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
        } else {
            AuthorizeUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
        }
    }
}

// =============================================================================
// FlowGateway Implementation for domain::Authorize
// =============================================================================

impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::Authorize
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            domain::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => Box::new(payment_gateway::DirectGateway),
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => Box::new(domain::Authorize),
        }
    }
}

// =============================================================================
// TODO Implementations - Using Macros
// =============================================================================

impl_payment_gateway_todo! {
    flow: domain::AuthorizeSessionToken,
    request_data: types::AuthorizeSessionTokenData,
    response_data: types::PaymentsResponseData,
    reason: "UCS GRPC endpoint for session tokens not available - decision pending"
}

impl_payment_gateway_todo! {
    flow: domain::PreProcessing,
    request_data: types::PaymentsPreProcessingData,
    response_data: types::PaymentsResponseData,
    reason: "UCS GRPC endpoint for preprocessing not available - decision pending"
}

impl_payment_gateway_todo! {
    flow: domain::PostProcessing,
    request_data: types::PaymentsPostProcessingData,
    response_data: types::PaymentsResponseData,
    reason: "UCS GRPC endpoint for post-processing not available - decision pending"
}

impl_payment_gateway_todo! {
    flow: domain::CreateOrder,
    request_data: types::CreateOrderRequestData,
    response_data: types::PaymentsResponseData,
    reason: "UCS GRPC endpoint for order creation not available - decision pending"
}