//! PaymentGateway implementation for api::SetupMandate flow
//!
//! This module implements the PaymentGateway trait for the SetupMandate flow,
//! handling mandate registration via the payment_setup_mandate GRPC endpoint.

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionMode, ExecutionPath};
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
use crate::core::payments::gateway::RouterGatewayContext;

use super::helpers::{build_grpc_auth_metadata, build_merchant_reference_id, get_grpc_client};
use crate::{
    core::{
        payments::helpers,
        unified_connector_service::{
            handle_unified_connector_service_response_for_payment_register, ucs_logging_wrapper,
        },
    },
    routes::SessionState,
    types::{self, transformers::ForeignTryFrom},
};

// /// Gateway struct for api::SetupMandate flow
// #[derive(Debug, Clone, Copy)]
// pub struct SetupMandateGateway;

/// Implementation of PaymentGateway for api::SetupMandate flow
#[async_trait]
impl<PaymentData, RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
        RouterGatewayContext<'static, PaymentData>,
    > for domain::SetupMandate
where
    PaymentData: Clone + Send + Sync + 'static,
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::SetupMandate,
            RCD,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<
            domain::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext<'static, PaymentData>,
    ) -> CustomResult<
        RouterData<domain::SetupMandate, types::SetupMandateRequestData, types::PaymentsResponseData>,
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
        //  context
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


        let payment_data = context
            .payment_data;

        // Execute payment_setup_mandate GRPC call
        let updated_router_data = execute_payment_setup_mandate(
            state,
            router_data,
            payment_data,
            merchant_context,
            header_payload,
            lineage_ids,
            context.execution_path,
        )
        .await?;

        Ok(updated_router_data)
    }
}

/// Implementation of FlowGateway for api::SetupMandate
///
/// This allows the flow to provide its specific gateway based on execution path
impl<PaymentData, RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
        RouterGatewayContext<'static, PaymentData>,
    > for domain::SetupMandate
where
    PaymentData: Clone + Send + Sync + 'static,
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
            RouterGatewayContext<'static, PaymentData>,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => {
                Box::new(payment_gateway::DirectGateway)
            }
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => {
                Box::new(domain::SetupMandate)
            }
        }
    }
}

async fn execute_payment_setup_mandate<PaymentData>(
    state: &SessionState,
    router_data: &RouterData<
        domain::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    >,
    payment_data: &PaymentData,
    merchant_context: &MerchantContext,
    header_payload: &HeaderPayload,
    lineage_ids: LineageIds,
    execution_path: ExecutionPath,
) -> CustomResult<
    RouterData<domain::SetupMandate, types::SetupMandateRequestData, types::PaymentsResponseData>,
    ConnectorError,
>
where
    PaymentData: Clone + Send + Sync + 'static,
{
    todo!()
}

/// Execute payment_setup_mandate GRPC call
// async fn execute_payment_setup_mandate<PaymentData>(
//     state: &SessionState,
//     router_data: &RouterData<
//         domain::SetupMandate,
//         types::SetupMandateRequestData,
//         types::PaymentsResponseData,
//     >,
//     payment_data: &PaymentData,
//     merchant_context: &MerchantContext,
//     header_payload: &HeaderPayload,
//     lineage_ids: LineageIds,
//     execution_mode: ExecutionMode,
// ) -> CustomResult<
//     RouterData<domain::SetupMandate, types::SetupMandateRequestData, types::PaymentsResponseData>,
//     ConnectorError,
// >
// where
//     PaymentData: Clone + Send + Sync + 'static,
// {
//     // Get GRPC client
//     let client = get_grpc_client(state)?;

//     // Build GRPC request
//     let payment_register_request =
//         payments_grpc::PaymentServiceRegisterRequest::foreign_try_from(router_data)
//             .change_context(ConnectorError::RequestEncodingFailed)?;

//     // Build auth metadata
//     let connector_auth_metadata = build_grpc_auth_metadata_from_payment_data(
//         payment_data,
//         merchant_context,
//     )?;

//     // Build GRPC headers
//     let merchant_reference_id = build_merchant_reference_id(header_payload);

//     let headers_builder = state
//         .get_grpc_headers_ucs(execution_mode)
//         .external_vault_proxy_metadata(None)
//         .merchant_reference_id(merchant_reference_id)
//         .lineage_ids(lineage_ids);

//     // Execute GRPC call with logging wrapper
//     let updated_router_data = Box::pin(ucs_logging_wrapper(
//         router_data.clone(),
//         state,
//         payment_register_request,
//         headers_builder,
//         |mut router_data, payment_register_request, grpc_headers| async move {
//             let response = client
//                 .payment_setup_mandate(
//                     payment_register_request,
//                     connector_auth_metadata,
//                     grpc_headers,
//                 )
//                 .await
//                 .change_context(ConnectorError::ProcessingStepFailed(Some(
//                     "Failed to setup mandate".to_string().into(),
//                 )))?;

//             let payment_register_response = response.into_inner();

//             let (router_data_response, status_code) =
//                 handle_unified_connector_service_response_for_payment_register(
//                     payment_register_response.clone(),
//                 )
//                 .change_context(ConnectorError::ResponseDeserializationFailed)?;

//             let router_data_response = router_data_response.map(|(response, status)| {
//                 router_data.status = status;
//                 response
//             });

//             router_data.response = router_data_response;
//             router_data.raw_connector_response = payment_register_response
//                 .raw_connector_response
//                 .clone()
//                 .map(Secret::new);
//             router_data.connector_http_status_code = Some(status_code);

//             Ok((router_data, payment_register_response))
//         },
//     ))
//     .await
//     .change_context(ConnectorError::ProcessingStepFailed(Some(
//         "UCS logging wrapper failed".to_string().into(),
//     )))?;

//     Ok(updated_router_data)
// }

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