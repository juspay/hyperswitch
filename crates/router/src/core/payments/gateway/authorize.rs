//! PaymentGateway implementation for api::Authorize flow
//!
//! This module implements the PaymentGateway trait for the Authorize flow,
//! handling both regular payments (payment_authorize) and mandate payments (payment_repeat).

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionMode};
use common_utils::{errors::CustomResult, request::Request};
use error_stack::ResultExt;
use external_services::grpc_client::{
    self, unified_connector_service::ConnectorAuthMetadata, LineageIds,
};
use hyperswitch_domain_models::{
    router_flow_types as domain,
    merchant_context::MerchantContext, payments::HeaderPayload, router_data::RouterData,
};
use hyperswitch_interfaces::{
    api::{self, gateway as payment_gateway},
    api_client::ApiClientWrapper,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};
use masking::Secret;
use unified_connector_service_client::payments as payments_grpc;

use super::helpers::{build_grpc_auth_metadata, build_merchant_reference_id, get_grpc_client};
use crate::{
    core::{
        payments::helpers,
        unified_connector_service::{
            handle_unified_connector_service_response_for_payment_authorize,
            handle_unified_connector_service_response_for_payment_repeat, ucs_logging_wrapper,
        },
    },
    routes::SessionState,
    types::{self, transformers::ForeignTryFrom},
};

// /// Gateway struct for api::Authorize flow
// #[derive(Debug, Clone, Copy)]
// pub struct AuthorizeGateway;

/// Implementation of PaymentGateway for api::Authorize flow
#[async_trait]
impl<PaymentData, RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
        PaymentData,
    > for domain::Authorize
where
    PaymentData: Clone + Send + Sync + 'static,
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,>,
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
        context: payment_gateway::GatewayExecutionContext<'_, domain::Authorize, PaymentData>,
    ) -> CustomResult<
        RouterData<domain::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        ConnectorError,
    > {
        // Extract required context
        let merchant_context = todo!();
        //  context
        //     .merchant_context
        //     .ok_or(ConnectorError::MissingRequiredField {
        //         field_name: "merchant_context",
        //     })?;

        let header_payload = todo!();
        // context
        //     .header_payload
        //     .ok_or(ConnectorError::MissingRequiredField {
        //         field_name: "header_payload",
        //     })?;

        
        let lineage_ids = todo!();
        
        // context
        //     .lineage_ids
        //     .ok_or(ConnectorError::MissingRequiredField {
        //         field_name: "lineage_ids",
        //     })?;

        // Extract payment_data to get merchant_connector_account
        let payment_data = context
            .payment_data
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "payment_data",
            })?;

        // Determine which GRPC endpoint to call based on mandate_id
        let updated_router_data = if router_data.request.mandate_id.is_some() {
            // Call payment_repeat for mandate payments
            execute_payment_repeat(
                state,
                router_data,
                payment_data,
                merchant_context,
                header_payload,
                lineage_ids,
                context.execution_mode,
            )
            .await?
        } else {
            // Call payment_authorize for regular payments
            execute_payment_authorize(
                state,
                router_data,
                payment_data,
                merchant_context,
                header_payload,
                lineage_ids,
                context.execution_mode,
            )
            .await?
        };

        Ok(updated_router_data)
    }
}

/// Implementation of FlowGateway for api::Authorize
///
/// This allows the flow to provide its specific gateway based on execution path
impl<PaymentData, RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
        PaymentData,
    > for domain::Authorize
where
    PaymentData: Clone + Send + Sync + 'static,
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        execution_path: payment_gateway::GatewayExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
            PaymentData,
        >,
    > {
        match execution_path {
            payment_gateway::GatewayExecutionPath::Direct => {
                Box::new(payment_gateway::DirectGateway)
            }
            payment_gateway::GatewayExecutionPath::UnifiedConnectorService
            | payment_gateway::GatewayExecutionPath::ShadowUnifiedConnectorService => {
                Box::new(domain::Authorize)
            }
        }
    }
}

/// Execute payment_authorize GRPC call
async fn execute_payment_authorize<PaymentData>(
    state: &SessionState,
    router_data: &RouterData<
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    >,
    payment_data: &PaymentData,
    merchant_context: &MerchantContext,
    header_payload: &HeaderPayload,
    lineage_ids: LineageIds,
    execution_mode: ExecutionMode,
) -> CustomResult<
    RouterData<domain::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ConnectorError,
>
where
    PaymentData: Clone + Send + Sync + 'static,
{
    todo!();
    // Get GRPC client
    // let client = get_grpc_client(state)?;

    // // Build GRPC request
    // let payment_authorize_request =
    //     payments_grpc::PaymentServiceAuthorizeRequest::foreign_try_from(router_data)
    //         .change_context(ConnectorError::RequestEncodingFailed)?;

    // // Build auth metadata
    // let connector_auth_metadata = build_grpc_auth_metadata_from_payment_data(
    //     payment_data,
    //     merchant_context,
    // )?;

    // // Build GRPC headers
    // let merchant_order_reference_id = build_merchant_reference_id(header_payload);

    // let headers_builder = state
    //     .get_grpc_headers_ucs(execution_mode)
    //     .external_vault_proxy_metadata(None)
    //     .merchant_reference_id(merchant_order_reference_id)
    //     .lineage_ids(lineage_ids);

    // // Execute GRPC call with logging wrapper
    // let updated_router_data = Box::pin(ucs_logging_wrapper(
    //     router_data.clone(),
    //     state,
    //     payment_authorize_request,
    //     headers_builder,
    //     |mut router_data, payment_authorize_request, grpc_headers| async move {
    //         let response = client
    //             .payment_authorize(
    //                 payment_authorize_request,
    //                 connector_auth_metadata,
    //                 grpc_headers,
    //             )
    //             .await
    //             .change_context(ConnectorError::ProcessingStepFailed(Some(
    //                 "Failed to authorize payment".to_string().into(),
    //             )))?;

    //         let payment_authorize_response = response.into_inner();

    //         let (router_data_response, status_code) =
    //             handle_unified_connector_service_response_for_payment_authorize(
    //                 payment_authorize_response.clone(),
    //             )
    //             .change_context(ConnectorError::ResponseDeserializationFailed)?;

    //         let router_data_response = router_data_response.map(|(response, status)| {
    //             router_data.status = status;
    //             response
    //         });

    //         router_data.response = router_data_response;
    //         router_data.raw_connector_response = payment_authorize_response
    //             .raw_connector_response
    //             .clone()
    //             .map(Secret::new);
    //         router_data.connector_http_status_code = Some(status_code);

    //         Ok((router_data, payment_authorize_response))
    //     },
    // ))
    // .await
    // .change_context(ConnectorError::ProcessingStepFailed(Some(
    //     "UCS logging wrapper failed".to_string().into(),
    // )))?;

    // Ok(updated_router_data)
}

/// Execute payment_repeat GRPC call
async fn execute_payment_repeat<PaymentData>(
    state: &SessionState,
    router_data: &RouterData<
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    >,
    payment_data: &PaymentData,
    merchant_context: &MerchantContext,
    header_payload: &HeaderPayload,
    lineage_ids: LineageIds,
    execution_mode: ExecutionMode,
) -> CustomResult<
    RouterData<domain::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ConnectorError,
>
where
    PaymentData: Clone + Send + Sync + 'static,
{
    todo!();
    // // Get GRPC client
    // let client = get_grpc_client(state)?;

    // // Build GRPC request
    // let payment_repeat_request =
    //     payments_grpc::PaymentServiceRepeatEverythingRequest::foreign_try_from(router_data)
    //         .change_context(ConnectorError::RequestEncodingFailed)?;

    // // Build auth metadata
    // let connector_auth_metadata = build_grpc_auth_metadata_from_payment_data(
    //     payment_data,
    //     merchant_context,
    // )?;

    // // Build GRPC headers
    // let merchant_order_reference_id = build_merchant_reference_id(header_payload);

    // let headers_builder = state
    //     .get_grpc_headers_ucs(execution_mode)
    //     .external_vault_proxy_metadata(None)
    //     .merchant_reference_id(merchant_order_reference_id)
    //     .lineage_ids(lineage_ids);

    // // Execute GRPC call with logging wrapper
    // let updated_router_data = Box::pin(ucs_logging_wrapper(
    //     router_data.clone(),
    //     state,
    //     payment_repeat_request,
    //     headers_builder,
    //     |mut router_data, payment_repeat_request, grpc_headers| async move {
    //         let response = client
    //             .payment_repeat(payment_repeat_request, connector_auth_metadata, grpc_headers)
    //             .await
    //             .change_context(ConnectorError::ProcessingStepFailed(Some(
    //                 "Failed to repeat payment".to_string().into(),
    //             )))?;

    //         let payment_repeat_response = response.into_inner();

    //         let (router_data_response, status_code) =
    //             handle_unified_connector_service_response_for_payment_repeat(
    //                 payment_repeat_response.clone(),
    //             )
    //             .change_context(ConnectorError::ResponseDeserializationFailed)?;

    //         let router_data_response = router_data_response.map(|(response, status)| {
    //             router_data.status = status;
    //             response
    //         });

    //         router_data.response = router_data_response;
    //         router_data.raw_connector_response = payment_repeat_response
    //             .raw_connector_response
    //             .clone()
    //             .map(Secret::new);
    //         router_data.connector_http_status_code = Some(status_code);

    //         Ok((router_data, payment_repeat_response))
    //     },
    // ))
    // .await
    // .change_context(ConnectorError::ProcessingStepFailed(Some(
    //     "UCS logging wrapper failed".to_string().into(),
    // )))?;

    // Ok(updated_router_data)
}

/// Helper to build GRPC auth metadata from payment data
/// This is a temporary implementation that needs to be updated based on actual PaymentData structure
fn build_grpc_auth_metadata_from_payment_data<PaymentData>(
    _payment_data: &PaymentData,
    _merchant_context: &MerchantContext,
) -> CustomResult<ConnectorAuthMetadata, ConnectorError>
where
    PaymentData: Clone + Send + Sync + 'static,
{
    // TODO: Extract merchant_connector_account from payment_data
    // This requires knowing the structure of PaymentData
    // For now, we'll return an error indicating this needs to be implemented

    // Placeholder implementation - needs to be replaced with actual extraction logic
    // The actual implementation should:
    // 1. Extract merchant_connector_account from payment_data
    // 2. Call build_grpc_auth_metadata with the extracted account

    Err(ConnectorError::NotImplemented(
        "build_grpc_auth_metadata_from_payment_data needs PaymentData structure implementation"
            .to_string(),
    )
    .into())
}

/// Implementation of PaymentGateway for domain::AuthorizeSessionToken flow with todo!()
#[async_trait]
impl<PaymentData, RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::AuthorizeSessionToken,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,
        PaymentData,
    > for domain::AuthorizeSessionToken
where
    PaymentData: Clone + Send + Sync + 'static,
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::AuthorizeSessionToken,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,>,
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
        _context: payment_gateway::GatewayExecutionContext<'_, domain::AuthorizeSessionToken, PaymentData>,
    ) -> CustomResult<
        RouterData<
            domain::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
        ConnectorError,
    > {
        todo!();
    }
}

/// Implementation of PaymentGateway for domain::PreProcessing flow with todo!()
#[async_trait]
impl<PaymentData, RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
        PaymentData,
    > for domain::PreProcessing
where
    PaymentData: Clone + Send + Sync + 'static,
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,>,
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
        _context: payment_gateway::GatewayExecutionContext<'_, domain::PreProcessing, PaymentData>,
    ) -> CustomResult<
        RouterData<
            domain::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
        ConnectorError,
    > {
        todo!();
    }
}

/// Implementation of PaymentGateway for domain::PostProcessing flow with todo!()
#[async_trait]
impl<PaymentData, RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::PostProcessing,
        types::PaymentsPostProcessingData,
        types::PaymentsResponseData,
        PaymentData,
    > for domain::PostProcessing
where
    PaymentData: Clone + Send + Sync + 'static,
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::PostProcessing,
        types::PaymentsPostProcessingData,
        types::PaymentsResponseData,>,
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
        _context: payment_gateway::GatewayExecutionContext<'_, domain::PostProcessing, PaymentData>,
    ) -> CustomResult<
        RouterData<
            domain::PostProcessing,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
        >,
        ConnectorError,
    > {
        todo!();
    }
}

/// Implementation of FlowGateway for api::AuthorizeSessionToken with todo!()
impl<PaymentData, RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,
        PaymentData,
    > for domain::AuthorizeSessionToken
where
    PaymentData: Clone + Send + Sync + 'static,
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::AuthorizeSessionToken,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        _execution_path: payment_gateway::GatewayExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
            PaymentData,
        >,
    > {
        todo!();
    }
}

/// Implementation of FlowGateway for api::PreProcessing with todo!()
impl<PaymentData, RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
        PaymentData,
    > for domain::PreProcessing
where
    PaymentData: Clone + Send + Sync + 'static,
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        _execution_path: payment_gateway::GatewayExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
            PaymentData,
        >,
    > {
        todo!();
    }
}

/// Implementation of FlowGateway for api::PostProcessing with todo!()
impl<PaymentData, RCD>
    payment_gateway::FlowGateway<   
        SessionState,
        RCD,
        types::PaymentsPostProcessingData,
        types::PaymentsResponseData,
        PaymentData,
    > for domain::PostProcessing
where
    PaymentData: Clone + Send + Sync + 'static,
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<  
        domain::PostProcessing,
        types::PaymentsPostProcessingData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        _execution_path: payment_gateway::GatewayExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
            PaymentData,
        >,
    > {
        todo!();
    }
}

/// Implementation of PaymentGateway for domain::CreateOrder flow with todo!()
#[async_trait]
impl<PaymentData, RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::CreateOrder,
        types::CreateOrderRequestData,
        types::PaymentsResponseData,
        PaymentData,
    > for domain::CreateOrder
where
    PaymentData: Clone + Send + Sync + 'static,
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::CreateOrder,
        types::CreateOrderRequestData,
        types::PaymentsResponseData,>,
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
        _context: payment_gateway::GatewayExecutionContext<'_, domain::CreateOrder, PaymentData>,
    ) -> CustomResult<
        RouterData<
            domain::CreateOrder,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
        >,
        ConnectorError,
    > {
        todo!();
    }
}

/// Implementation of FlowGateway for api::CreateOrder with todo!()
impl<PaymentData, RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::CreateOrderRequestData,
        types::PaymentsResponseData,
        PaymentData,
    > for domain::CreateOrder
where
    PaymentData: Clone + Send + Sync + 'static,
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::CreateOrder,
        types::CreateOrderRequestData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        _execution_path: payment_gateway::GatewayExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
            PaymentData,
        >,
    > {
        todo!();
    }
}

