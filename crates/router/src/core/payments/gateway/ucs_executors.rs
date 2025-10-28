//! UCS Flow Executors
//!
//! This module contains concrete implementations of the UCS traits for different payment flows.
//! Each executor handles a specific GRPC endpoint (payment_authorize, payment_repeat, payment_get, payment_setup_mandate).

use async_trait::async_trait;
use common_enums::{AttemptStatus, ExecutionMode};
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use external_services::grpc_client::{self, unified_connector_service::UnifiedConnectorServiceClient};
use std::borrow::Borrow;
use tonic;
use hyperswitch_domain_models::{
    merchant_context::MerchantContext,
    payments::HeaderPayload,
    router_data::{ErrorResponse, RouterData},
    router_flow_types as domain, router_response_types::PaymentsResponseData,
};
use hyperswitch_interfaces::{
    errors::ConnectorError,
    unified_connector_service::{
        handle_unified_connector_service_response_for_payment_get,
        UcsContext, UcsExecutionContextProvider, UcsFlowExecutor, UcsGrpcExecutor, UcsRequestTransformer,
        UcsResponseHandler, UcsStateProvider,
    },
};
use masking::Secret;
use unified_connector_service_client::payments as payments_grpc;

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

use super::{
    helpers::prepare_ucs_infrastructure,
    ucs_context::RouterUcsContext,
    ucs_execution_context::RouterUcsExecutionContext,
};

// =============================================================================
// Generic UCS Executor Function
// =============================================================================

/// Generic UCS executor function
///
/// This function provides a reusable implementation for executing UCS flows.
/// It handles the complete flow:
/// 1. Prepares infrastructure (client, auth, headers)
/// 2. Transforms the request
/// 3. Executes the GRPC call with logging
/// 4. Handles the response
/// 5. Updates and returns RouterData
///
/// # Type Parameters
/// - `F`: Flow type (e.g., Authorize, PSync)
/// - `Exe`: Executor type that implements all required UCS traits
/// - `Req`: Request data type
/// - `Resp`: Response data type
/// - `GrpcReq`: GRPC request type
/// - `GrpcResp`: GRPC response type
///
/// # Arguments
/// - `state`: Session state
/// - `router_data`: RouterData to process
/// - `execution_context`: Execution context with merchant info, headers, etc.
///
/// # Returns
/// Updated RouterData with response from UCS
pub async fn ucs_executor<'a, F, Exe, Req, Resp, GrpcReq, GrpcResp>(
    state: &SessionState,
    router_data: &RouterData<F, Req, Resp>,
    execution_context: RouterUcsExecutionContext<'a>,
) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>
where
    F: Send + Sync + Clone + std::fmt::Debug + 'static,
    Req: Send + Sync + Clone + std::fmt::Debug + 'static,
    Resp: Send + Sync + Clone + std::fmt::Debug + 'static,
    GrpcReq: serde::Serialize + std::fmt::Debug + Send,
    GrpcResp: serde::Serialize + std::fmt::Debug + Clone + Send,
    Exe: UcsRequestTransformer<F, Req, Resp, GrpcRequest = GrpcReq>
        + UcsResponseHandler<GrpcResp, Resp>
        + UcsGrpcExecutor<UnifiedConnectorServiceClient, RouterUcsContext, GrpcReq, GrpcResp, GrpcResponse = tonic::Response<GrpcResp>>
        + UcsFlowExecutor<F, Req, Resp, SessionState, GrpcRequest = GrpcReq, GrpcResponse = GrpcResp>,
    for<'b> Exe: UcsFlowExecutor<F, Req, Resp, SessionState, ExecCtx<'b> = RouterUcsExecutionContext<'b>>,
{
    let (client, auth, headers_builder) = prepare_ucs_infrastructure(
        state,
        execution_context.merchant_context(),
        execution_context.header_payload(),
        execution_context.lineage_ids(),
        execution_context.merchant_connector_account(),
        execution_context.execution_mode(),
    )?;

    let grpc_request = Exe::transform_request(router_data)?;

    // Extract lineage_ids before the closure to avoid lifetime issues
    let lineage_ids = execution_context.lineage_ids();

    let updated_router_data = ucs_logging_wrapper(
        (*router_data).clone(),
        state,
        grpc_request,
        headers_builder,
        |mut router_data, grpc_request, grpc_headers| async move {
            let context = RouterUcsContext::new(auth, grpc_headers, lineage_ids);

            let response = Exe::execute_grpc_call(&client, grpc_request, context)
                .await
                .change_context(
                    hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError,
                )?;

            let grpc_response = response.into_inner();

            let (router_data_response, status_code) =
                Exe::handle_response(grpc_response.clone()).change_context(
                    hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError,
                )?;

            let router_data_response = router_data_response.map(|(response, status)| {
                router_data.status = status;
                response
            });
            router_data.response = router_data_response;
            router_data.connector_http_status_code = Some(status_code);

            Ok((router_data, grpc_response))
        },
    )
    .await
    .change_context(ConnectorError::ProcessingStepFailed(None))?;

    Ok(updated_router_data)
}

// // =============================================================================
// // Concrete Executor Implementations
// // =============================================================================

// /// Executor for payment_authorize GRPC endpoint
// #[derive(Debug, Clone, Copy)]
// pub struct AuthorizeUcsExecutor;

// impl UcsRequestTransformer<domain::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
//     for AuthorizeUcsExecutor
// {
//     type GrpcRequest = payments_grpc::PaymentServiceAuthorizeRequest;

//     fn transform_request(
//         router_data: &RouterData<
//             domain::Authorize,
//             types::PaymentsAuthorizeData,
//             types::PaymentsResponseData,
//         >,
//     ) -> CustomResult<Self::GrpcRequest, ConnectorError> {
//         payments_grpc::PaymentServiceAuthorizeRequest::foreign_try_from(router_data)
//             .change_context(ConnectorError::RequestEncodingFailed)
//     }
// }

// impl UcsResponseHandler<payments_grpc::PaymentServiceAuthorizeResponse, PaymentsResponseData> for AuthorizeUcsExecutor {
//     fn handle_response(
//         response: payments_grpc::PaymentServiceAuthorizeResponse,
//     ) -> CustomResult<
//         (
//             Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>,
//             u16,
//         ),
//         ConnectorError,
//     > {
//         handle_unified_connector_service_response_for_payment_authorize(response)
//             .change_context(ConnectorError::ResponseHandlingFailed)
//     }
// }

// #[async_trait]
// impl UcsGrpcExecutor<
//         UnifiedConnectorServiceClient,
//         RouterUcsContext,
//         payments_grpc::PaymentServiceAuthorizeRequest,
//         payments_grpc::PaymentServiceAuthorizeResponse,
//     > for AuthorizeUcsExecutor
// {
//     type GrpcResponse = tonic::Response<payments_grpc::PaymentServiceAuthorizeResponse>;

//     async fn execute_grpc_call(
//         client: &UnifiedConnectorServiceClient,
//         request: payments_grpc::PaymentServiceAuthorizeRequest,
//         context: RouterUcsContext,
//     ) -> CustomResult<Self::GrpcResponse, ConnectorError>
//     {
//         client
//             .payment_authorize(request, context.auth(), context.headers())
//             .await
//             .change_context(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError)
//             .change_context(ConnectorError::ProcessingStepFailed(None))
//     }
// }

// #[async_trait]
// impl UcsFlowExecutor<domain::Authorize, types::PaymentsAuthorizeData, PaymentsResponseData> for AuthorizeUcsExecutor {
//     type GrpcRequest = payments_grpc::PaymentServiceAuthorizeRequest;
//     type GrpcResponse = payments_grpc::PaymentServiceAuthorizeResponse;

//     async fn execute_ucs_flow<State, ExecCtx>(
//         state: &State,
//         router_data: &RouterData<domain::Authorize, types::PaymentsAuthorizeData, PaymentsResponseData>,
//         execution_context: ExecCtx,
//     ) -> CustomResult<RouterData<domain::Authorize, types::PaymentsAuthorizeData, PaymentsResponseData>, ConnectorError>
//     where
//         State: UcsStateProvider + 'static,
//         ExecCtx: UcsExecutionContextProvider,
//         Self::GrpcRequest: serde::Serialize + std::fmt::Debug,
//         Self::GrpcResponse: std::fmt::Debug,
//     {
//         // Downcast state to SessionState
//         let session_state = (state as &dyn std::any::Any)
//             .downcast_ref::<SessionState>()
//             .ok_or(ConnectorError::ProcessingStepFailed(Some(
//                 "State must be SessionState".to_string().into()
//             )))?;
        
//         // Step 1-3: Prepare common infrastructure from execution context
//         let (client, auth, headers_builder) = prepare_ucs_infrastructure(
//             session_state,
//             execution_context.merchant_context().borrow(),
//             execution_context.header_payload().borrow(),
//             execution_context.lineage_ids(),
//             execution_context.merchant_connector_account(),
//             execution_context.execution_mode(),
//         )?;

//         // Step 4: Transform request
//         let grpc_request = Self::transform_request(router_data)?;

//         // Step 5-6: Execute with logging wrapper
//         let updated_router_data = ucs_logging_wrapper(
//             router_data.clone(),
//             session_state,
//             grpc_request,
//             headers_builder,
//             |mut router_data, grpc_request, grpc_headers| async move {
//                 // Create UCS context
//                 let context = create_ucs_context(auth, grpc_headers, execution_context.lineage_ids());

//                 // Polymorphic GRPC call
//                 let response = Self::execute_grpc_call(&client, grpc_request, context)
//                     .await?;

//                 let grpc_response = response.into_inner();

//                 // Polymorphic response handling
//                 let (router_data_response, status_code) =
//                     Self::handle_response(grpc_response.clone())?;

//                 // Update router_data with response
//                 let router_data_response = router_data_response.map(|(response, status)| {
//                     router_data.status = status;
//                     response
//                 });
//                 router_data.response = router_data_response;
//                 router_data.raw_connector_response = grpc_response
//                     .raw_connector_response
//                     .clone()
//                     .map(Secret::new);
//                 router_data.connector_http_status_code = Some(status_code);

//                 Ok((router_data, grpc_response))
//             },
//         )
//         .await?;

//         Ok(updated_router_data)
//     }
// }

// /// Executor for payment_repeat GRPC endpoint
// #[derive(Debug, Clone, Copy)]
// pub struct RepeatUcsExecutor;

// impl UcsRequestTransformer<domain::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
//     for RepeatUcsExecutor
// {
//     type GrpcRequest = payments_grpc::PaymentServiceRepeatEverythingRequest;

//     fn transform_request(
//         router_data: &RouterData<
//             domain::Authorize,
//             types::PaymentsAuthorizeData,
//             types::PaymentsResponseData,
//         >,
//     ) -> CustomResult<Self::GrpcRequest, ConnectorError> {
//         payments_grpc::PaymentServiceRepeatEverythingRequest::foreign_try_from(router_data)
//             .change_context(ConnectorError::RequestEncodingFailed)
//     }
// }

// impl UcsResponseHandler<payments_grpc::PaymentServiceRepeatEverythingResponse, PaymentsResponseData>
//     for RepeatUcsExecutor
// {
//     fn handle_response(
//         response: payments_grpc::PaymentServiceRepeatEverythingResponse,
//     ) -> CustomResult<
//         (
//             Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>,
//             u16,
//         ),
//         ConnectorError,
//     > {
//         handle_unified_connector_service_response_for_payment_repeat(response)
//             .change_context(ConnectorError::ResponseHandlingFailed)
//     }
// }

// #[async_trait]
// impl UcsGrpcExecutor<
//         UnifiedConnectorServiceClient,
//         RouterUcsContext,
//         payments_grpc::PaymentServiceRepeatEverythingRequest,
//         payments_grpc::PaymentServiceRepeatEverythingResponse,
//     > for RepeatUcsExecutor
// {
//     type GrpcResponse = tonic::Response<payments_grpc::PaymentServiceRepeatEverythingResponse>;

//     async fn execute_grpc_call(
//         client: &UnifiedConnectorServiceClient,
//         request: payments_grpc::PaymentServiceRepeatEverythingRequest,
//         context: RouterUcsContext,
//     ) -> CustomResult<Self::GrpcResponse, ConnectorError>
//     {
//         client
//             .payment_repeat(request, context.auth(), context.headers())
//             .await
//             .change_context(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError)
//             .change_context(ConnectorError::ProcessingStepFailed(None))
//     }
// }

// #[async_trait]
// impl UcsFlowExecutor<domain::Authorize, types::PaymentsAuthorizeData, PaymentsResponseData> for RepeatUcsExecutor {
//     type GrpcRequest = payments_grpc::PaymentServiceRepeatEverythingRequest;
//     type GrpcResponse = payments_grpc::PaymentServiceRepeatEverythingResponse;

//     async fn execute_ucs_flow<State, ExecCtx>(
//         state: &State,
//         router_data: &RouterData<domain::Authorize, types::PaymentsAuthorizeData, PaymentsResponseData>,
//         execution_context: ExecCtx,
//     ) -> CustomResult<RouterData<domain::Authorize, types::PaymentsAuthorizeData, PaymentsResponseData>, ConnectorError>
//     where
//         State: UcsStateProvider + 'static,
//         ExecCtx: UcsExecutionContextProvider,
//         Self::GrpcRequest: serde::Serialize + std::fmt::Debug,
//         Self::GrpcResponse: std::fmt::Debug,
//     {
//         // Downcast state to SessionState
//         let session_state = (state as &dyn std::any::Any)
//             .downcast_ref::<SessionState>()
//             .ok_or(ConnectorError::ProcessingStepFailed(Some(
//                 "State must be SessionState".to_string().into()
//             )))?;
        
//         // Step 1-3: Prepare common infrastructure from execution context
//         let (client, auth, headers_builder) = prepare_ucs_infrastructure(
//             session_state,
//             execution_context.merchant_context().borrow(),
//             execution_context.header_payload().borrow(),
//             execution_context.lineage_ids(),
//             execution_context.merchant_connector_account(),
//             execution_context.execution_mode(),
//         )?;

//         // Step 4: Transform request
//         let grpc_request = Self::transform_request(router_data)?;

//         // Step 5-6: Execute with logging wrapper
//         let updated_router_data = ucs_logging_wrapper(
//             router_data.clone(),
//             session_state,
//             grpc_request,
//             headers_builder,
//             |mut router_data, grpc_request, grpc_headers| async move {
//                 // Create UCS context
//                 let context = create_ucs_context(auth, grpc_headers, execution_context.lineage_ids());

//                 // Polymorphic GRPC call
//                 let response = Self::execute_grpc_call(&client, grpc_request, context)
//                     .await?;

//                 let grpc_response = response.into_inner();

//                 // Polymorphic response handling
//                 let (router_data_response, status_code) =
//                     Self::handle_response(grpc_response.clone())?;

//                 // Update router_data with response
//                 let router_data_response = router_data_response.map(|(response, status)| {
//                     router_data.status = status;
//                     response
//                 });
//                 router_data.response = router_data_response;
//                 router_data.raw_connector_response = grpc_response
//                     .raw_connector_response
//                     .clone()
//                     .map(Secret::new);
//                 router_data.connector_http_status_code = Some(status_code);

//                 Ok((router_data, grpc_response))
//             },
//         )
//         .await?;

//         Ok(updated_router_data)
//     }
// }

// /// Executor for payment_get GRPC endpoint (PSync)
// #[derive(Debug, Clone, Copy)]
// pub struct PSyncUcsExecutor;

// impl UcsRequestTransformer<domain::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
//     for PSyncUcsExecutor
// {
//     type GrpcRequest = payments_grpc::PaymentServiceGetRequest;

//     fn transform_request(
//         router_data: &RouterData<domain::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
//     ) -> CustomResult<Self::GrpcRequest, ConnectorError> {
//         payments_grpc::PaymentServiceGetRequest::foreign_try_from(router_data)
//             .change_context(ConnectorError::RequestEncodingFailed)
//     }
// }

// impl UcsResponseHandler<payments_grpc::PaymentServiceGetResponse, PaymentsResponseData> for PSyncUcsExecutor {
//     fn handle_response(
//         response: payments_grpc::PaymentServiceGetResponse,
//     ) -> CustomResult<
//         (
//             Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>,
//             u16,
//         ),
//         ConnectorError,
//     > {
//         handle_unified_connector_service_response_for_payment_get(response)
//             .change_context(ConnectorError::ResponseHandlingFailed)
//     }
// }

// #[async_trait]
// impl UcsGrpcExecutor<
//         UnifiedConnectorServiceClient,
//         RouterUcsContext,
//         payments_grpc::PaymentServiceGetRequest,
//         payments_grpc::PaymentServiceGetResponse,
//     > for PSyncUcsExecutor
// {
//     type GrpcResponse = tonic::Response<payments_grpc::PaymentServiceGetResponse>;

//     async fn execute_grpc_call(
//         client: &UnifiedConnectorServiceClient,
//         request: payments_grpc::PaymentServiceGetRequest,
//         context: RouterUcsContext,
//     ) -> CustomResult<Self::GrpcResponse, ConnectorError> {
//         client
//             .payment_get(request, context.auth(), context.headers())
//             .await
//             .change_context(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError)
//             .change_context(ConnectorError::ProcessingStepFailed(None))
//     }
// }

// #[async_trait]
// impl UcsFlowExecutor<domain::PSync, types::PaymentsSyncData, PaymentsResponseData> for PSyncUcsExecutor {
//     type GrpcRequest = payments_grpc::PaymentServiceGetRequest;
//     type GrpcResponse = payments_grpc::PaymentServiceGetResponse;

//     async fn execute_ucs_flow<State, ExecCtx>(
//         state: &State,
//         router_data: &RouterData<domain::PSync, types::PaymentsSyncData, PaymentsResponseData>,
//         execution_context: ExecCtx,
//     ) -> CustomResult<RouterData<domain::PSync, types::PaymentsSyncData, PaymentsResponseData>, ConnectorError>
//     where
//         State: UcsStateProvider + 'static,
//         ExecCtx: UcsExecutionContextProvider,
//         Self::GrpcRequest: serde::Serialize + std::fmt::Debug,
//         Self::GrpcResponse: std::fmt::Debug,
//     {
//         // Downcast state to SessionState
//         let session_state = (state as &dyn std::any::Any)
//             .downcast_ref::<SessionState>()
//             .ok_or(ConnectorError::ProcessingStepFailed(Some(
//                 "State must be SessionState".to_string().into()
//             )))?;
        
//         // Step 1-3: Prepare common infrastructure from execution context
//         let (client, auth, headers_builder) = prepare_ucs_infrastructure(
//             session_state,
//             execution_context.merchant_context().borrow(),
//             execution_context.header_payload().borrow(),
//             execution_context.lineage_ids(),
//             execution_context.merchant_connector_account(),
//             execution_context.execution_mode(),
//         )?;

//         // Step 4: Transform request
//         let grpc_request = Self::transform_request(router_data)?;

//         // Step 5-6: Execute with logging wrapper
//         let updated_router_data = ucs_logging_wrapper(
//             router_data.clone(),
//             session_state,
//             grpc_request,
//             headers_builder,
//             |mut router_data, grpc_request, grpc_headers| async move {
//                 // Create UCS context
//                 let context = create_ucs_context(auth, grpc_headers, execution_context.lineage_ids());

//                 // Polymorphic GRPC call
//                 let response = Self::execute_grpc_call(&client, grpc_request, context)
//                     .await?;

//                 let grpc_response = response.into_inner();

//                 // Polymorphic response handling
//                 let (router_data_response, status_code) =
//                     Self::handle_response(grpc_response.clone())?;

//                 // Update router_data with response
//                 let router_data_response = router_data_response.map(|(response, status)| {
//                     router_data.status = status;
//                     response
//                 });
//                 router_data.response = router_data_response;
//                 router_data.raw_connector_response = grpc_response
//                     .raw_connector_response
//                     .clone()
//                     .map(Secret::new);
//                 router_data.connector_http_status_code = Some(status_code);

//                 Ok((router_data, grpc_response))
//             },
//         )
//         .await?;

//         Ok(updated_router_data)
//     }
// }

// /// Executor for payment_setup_mandate GRPC endpoint
// #[derive(Debug, Clone, Copy)]
// pub struct SetupMandateUcsExecutor;

// impl UcsRequestTransformer<
//         domain::SetupMandate,
//         types::SetupMandateRequestData,
//         types::PaymentsResponseData,
//     > for SetupMandateUcsExecutor
// {
//     type GrpcRequest = payments_grpc::PaymentServiceRegisterRequest;

//     fn transform_request(
//         router_data: &RouterData<
//             domain::SetupMandate,
//             types::SetupMandateRequestData,
//             types::PaymentsResponseData,
//         >,
//     ) -> CustomResult<Self::GrpcRequest, ConnectorError> {
//         payments_grpc::PaymentServiceRegisterRequest::foreign_try_from(router_data)
//             .change_context(ConnectorError::RequestEncodingFailed)
//     }
// }

// impl UcsResponseHandler<payments_grpc::PaymentServiceRegisterResponse, PaymentsResponseData>
//     for SetupMandateUcsExecutor
// {
//     fn handle_response(
//         response: payments_grpc::PaymentServiceRegisterResponse,
//     ) -> CustomResult<
//         (
//             Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>,
//             u16,
//         ),
//         ConnectorError,
//     > {
//         crate::core::unified_connector_service::handle_unified_connector_service_response_for_payment_register(response)
//             .change_context(ConnectorError::ResponseHandlingFailed)
//     }
// }

// #[async_trait]
// impl UcsGrpcExecutor<
//         UnifiedConnectorServiceClient,
//         RouterUcsContext,
//         payments_grpc::PaymentServiceRegisterRequest,
//         payments_grpc::PaymentServiceRegisterResponse,
//     > for SetupMandateUcsExecutor
// {
//     type GrpcResponse = tonic::Response<payments_grpc::PaymentServiceRegisterResponse>;

//     async fn execute_grpc_call(
//         client: &UnifiedConnectorServiceClient,
//         request: payments_grpc::PaymentServiceRegisterRequest,
//         context: RouterUcsContext,
//     ) -> CustomResult<Self::GrpcResponse, ConnectorError>
//     {
//         client
//             .payment_setup_mandate(request, context.auth(), context.headers())
//             .await
//             .change_context(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError)
//             .change_context(ConnectorError::ProcessingStepFailed(None))
//     }
// }

// #[async_trait]
// impl UcsFlowExecutor<domain::SetupMandate, types::SetupMandateRequestData, PaymentsResponseData> for SetupMandateUcsExecutor {
//     type GrpcRequest = payments_grpc::PaymentServiceRegisterRequest;
//     type GrpcResponse = payments_grpc::PaymentServiceRegisterResponse;

//     async fn execute_ucs_flow<State, ExecCtx>(
//         state: &State,
//         router_data: &RouterData<domain::SetupMandate, types::SetupMandateRequestData, PaymentsResponseData>,
//         execution_context: ExecCtx,
//     ) -> CustomResult<RouterData<domain::SetupMandate, types::SetupMandateRequestData, PaymentsResponseData>, ConnectorError>
//     where
//         State: UcsStateProvider + 'static,
//         ExecCtx: UcsExecutionContextProvider,
//         Self::GrpcRequest: serde::Serialize + std::fmt::Debug,
//         Self::GrpcResponse: std::fmt::Debug,
//     {
//         // Downcast state to SessionState
//         let session_state = (state as &dyn std::any::Any)
//             .downcast_ref::<SessionState>()
//             .ok_or(ConnectorError::ProcessingStepFailed(Some(
//                 "State must be SessionState".to_string().into()
//             )))?;
        
//         // Step 1-3: Prepare common infrastructure from execution context
//         let (client, auth, headers_builder) = prepare_ucs_infrastructure(
//             session_state,
//             execution_context.merchant_context().borrow(),
//             execution_context.header_payload().borrow(),
//             execution_context.lineage_ids(),
//             execution_context.merchant_connector_account(),
//             execution_context.execution_mode(),
//         )?;

//         // Step 4: Transform request
//         let grpc_request = Self::transform_request(router_data)?;

//         // Step 5-6: Execute with logging wrapper
//         let updated_router_data = ucs_logging_wrapper(
//             router_data.clone(),
//             session_state,
//             grpc_request,
//             headers_builder,
//             |mut router_data, grpc_request, grpc_headers| async move {
//                 // Create UCS context
//                 let context = create_ucs_context(auth, grpc_headers, execution_context.lineage_ids());

//                 // Polymorphic GRPC call
//                 let response = Self::execute_grpc_call(&client, grpc_request, context)
//                     .await?;

//                 let grpc_response = response.into_inner();

//                 // Polymorphic response handling
//                 let (router_data_response, status_code) =
//                     Self::handle_response(grpc_response.clone())?;

//                 // Update router_data with response
//                 let router_data_response = router_data_response.map(|(response, status)| {
//                     router_data.status = status;
//                     response
//                 });
//                 router_data.response = router_data_response;
//                 router_data.raw_connector_response = grpc_response
//                     .raw_connector_response
//                     .clone()
//                     .map(Secret::new);
//                 router_data.connector_http_status_code = Some(status_code);

//                 Ok((router_data, grpc_response))
//             },
//         )
//         .await?;

//         Ok(updated_router_data)
//     }
// }