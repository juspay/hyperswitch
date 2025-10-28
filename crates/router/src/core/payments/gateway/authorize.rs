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
    router_data::{self, ErrorResponse, RouterData},
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
    ucs_executors::ucs_executor,
    ucs_execution_context::RouterUcsExecutionContext,
};
use crate::{
    core::unified_connector_service::{
        handle_unified_connector_service_response_for_payment_authorize, ucs_logging_wrapper,
    },
    routes::SessionState,
    types::{self, transformers::ForeignTryFrom},
};

// =============================================================================
// AuthorizeUcsExecutor - UCS Flow Executor for Authorize
// =============================================================================

#[derive(Debug, Clone, Copy)]
struct AuthorizeUcsExecutor;

// =============================================================================
// Trait Implementations for AuthorizeUcsExecutor
// =============================================================================

impl
    UcsRequestTransformer<
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for AuthorizeUcsExecutor
{
    type GrpcRequest = payments_grpc::PaymentServiceAuthorizeRequest;

    fn transform_request(
        router_data: &RouterData<
            domain::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> CustomResult<Self::GrpcRequest, ConnectorError> {
        payments_grpc::PaymentServiceAuthorizeRequest::foreign_try_from(router_data)
            .change_context(ConnectorError::RequestEncodingFailed)
    }
}

impl UcsResponseHandler<payments_grpc::PaymentServiceAuthorizeResponse, types::PaymentsResponseData> for AuthorizeUcsExecutor {
    fn handle_response(
        response: payments_grpc::PaymentServiceAuthorizeResponse,
    ) -> CustomResult<
        (
            Result<(types::PaymentsResponseData, common_enums::AttemptStatus), ErrorResponse>,
            u16,
        ),
        ConnectorError,
    > {
        handle_unified_connector_service_response_for_payment_authorize(response)
            .change_context(ConnectorError::ResponseHandlingFailed)
    }
}

#[async_trait]
impl
    UcsGrpcExecutor<
        UnifiedConnectorServiceClient,
        RouterUcsContext,
        payments_grpc::PaymentServiceAuthorizeRequest,
        payments_grpc::PaymentServiceAuthorizeResponse,
    > for AuthorizeUcsExecutor
{
    type GrpcResponse = tonic::Response<payments_grpc::PaymentServiceAuthorizeResponse>;

    async fn execute_grpc_call(
        client: &UnifiedConnectorServiceClient,
        request: payments_grpc::PaymentServiceAuthorizeRequest,
        context: RouterUcsContext,
    ) -> CustomResult<Self::GrpcResponse, ConnectorError> {
        client
            .payment_authorize(request, context.auth(), context.headers())
            .await
            .change_context(
                hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError,
            )
            .change_context(ConnectorError::ProcessingStepFailed(None))
    }
}

#[async_trait]
impl
    UcsFlowExecutor<
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
        SessionState,
    > for AuthorizeUcsExecutor
{
    type GrpcRequest = payments_grpc::PaymentServiceAuthorizeRequest;
    type GrpcResponse = payments_grpc::PaymentServiceAuthorizeResponse;
    type ExecCtx<'a> = RouterUcsExecutionContext<'a>;

    async fn execute_ucs_flow<'a>(
        state: &SessionState,
        router_data: &RouterData<
            domain::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        execution_context: RouterUcsExecutionContext<'a>,
    ) -> CustomResult<
        RouterData<domain::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        ConnectorError,
    >
    where
        Self::GrpcRequest: serde::Serialize + std::fmt::Debug,
        Self::GrpcResponse: std::fmt::Debug,
    {
        ucs_executor::<domain::Authorize, AuthorizeUcsExecutor, types::PaymentsAuthorizeData, types::PaymentsResponseData, _, _>(state, router_data, execution_context).await
    }
}

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
        // Determine which GRPC endpoint to call based on mandate_id
        // let updated_router_data = if router_data.request.mandate_id.is_some() {
        //     // Create execution context for payment_repeat
        //     let execution_context = RouterUcsExecutionContext::new(
        //         &context.merchant_context,
        //         &context.header_payload,
        //         context.lineage_ids,
        //         &context.merchant_connector_account,
        //         context.execution_mode,
        //     );
        //     // Call payment_repeat for mandate payments using trait-based executor
        //     RepeatUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await?
        // } else {
        //     // Create execution context for payment_authorize
        //     let execution_context = RouterUcsExecutionContext::new(
        //         &context.merchant_context,
        //         &context.header_payload,
        //         context.lineage_ids,
        //         &context.merchant_connector_account,
        //         context.execution_mode,
        //     );
        //     // Call payment_authorize for regular payments using trait-based executor
        //     AuthorizeUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await?
        // };
        // Ok(updated_router_data)

        // Temporary implementation - using only AuthorizeUcsExecutor until RepeatUcsExecutor is implemented
        let execution_context = RouterUcsExecutionContext::new(
            &context.merchant_context,
            &context.header_payload,
            context.lineage_ids,
            &context.merchant_connector_account,
            context.execution_mode,
        );
        AuthorizeUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
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
// TODO Implementations - Pending UCS GRPC Endpoints
// =============================================================================

#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::AuthorizeSessionToken,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::AuthorizeSessionToken
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            domain::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
{
    async fn execute(
        self: Box<Self>,
        _state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::AuthorizeSessionToken,
            RCD,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
        _router_data: &RouterData<
            domain::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        _context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<
            domain::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
        ConnectorError,
    > {
        todo!("UCS GRPC endpoint for session tokens not available - decision pending")
    }
}

impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::AuthorizeSessionToken
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            domain::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
{
    fn get_gateway(
        _execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        todo!("UCS GRPC endpoint for session tokens not available - decision pending")
    }
}

#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::PreProcessing
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            domain::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
{
    async fn execute(
        self: Box<Self>,
        _state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::PreProcessing,
            RCD,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
        _router_data: &RouterData<
            domain::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        _context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<
            domain::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
        ConnectorError,
    > {
        todo!("UCS GRPC endpoint for preprocessing not available - decision pending")
    }
}

impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::PreProcessing
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            domain::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
{
    fn get_gateway(
        _execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        todo!("UCS GRPC endpoint for preprocessing not available - decision pending")
    }
}

#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::PostProcessing,
        types::PaymentsPostProcessingData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::PostProcessing
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            domain::PostProcessing,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
        >,
{
    async fn execute(
        self: Box<Self>,
        _state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::PostProcessing,
            RCD,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
        >,
        _router_data: &RouterData<
            domain::PostProcessing,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        _context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<
            domain::PostProcessing,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
        >,
        ConnectorError,
    > {
        todo!("UCS GRPC endpoint for post-processing not available - decision pending")
    }
}

impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PaymentsPostProcessingData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::PostProcessing
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            domain::PostProcessing,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
        >,
{
    fn get_gateway(
        _execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        todo!("UCS GRPC endpoint for post-processing not available - decision pending")
    }
}

#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::CreateOrder,
        types::CreateOrderRequestData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::CreateOrder
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            domain::CreateOrder,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
        >,
{
    async fn execute(
        self: Box<Self>,
        _state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::CreateOrder,
            RCD,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
        >,
        _router_data: &RouterData<
            domain::CreateOrder,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        _context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<
            domain::CreateOrder,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
        >,
        ConnectorError,
    > {
        todo!("UCS GRPC endpoint for order creation not available - decision pending")
    }
}

impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::CreateOrderRequestData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::CreateOrder
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            domain::CreateOrder,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
        >,
{
    fn get_gateway(
        _execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        todo!("UCS GRPC endpoint for order creation not available - decision pending")
    }
}