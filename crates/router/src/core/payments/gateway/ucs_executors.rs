//! UCS Generic Executor Function
//!
//! This module contains the generic `ucs_executor` function that provides
//! a reusable implementation for executing UCS flows.
//!
//! Concrete executor implementations are now located in their respective flow files:
//! - `AuthorizeUcsExecutor` and `RepeatUcsExecutor` in `authorize.rs`
//! - `PSyncUcsExecutor` in `psync.rs`
//! - `SetupMandateUcsExecutor` in `setup_mandate.rs`

use async_trait::async_trait;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use external_services::grpc_client::unified_connector_service::UnifiedConnectorServiceClient;
use hyperswitch_domain_models::router_data::RouterData;
use hyperswitch_interfaces::{
    errors::ConnectorError,
    unified_connector_service::{
        UcsFlowExecutor, UcsGrpcExecutor, UcsRequestTransformer, UcsResponseHandler,
    },
};

use hyperswitch_interfaces::unified_connector_service::UcsExecutionContextProvider;

use super::{
    helpers::prepare_ucs_infrastructure,
    ucs_context::RouterUcsContext,
    ucs_execution_context::RouterUcsExecutionContext,
};
use crate::{core::unified_connector_service::ucs_logging_wrapper, routes::SessionState};

// =============================================================================
// Macro for Defining UCS Executors
// =============================================================================

/// Macro to define a UCS executor with all required trait implementations
///
/// This macro generates:
/// 1. UcsRequestTransformer - Transforms RouterData to GRPC request
/// 2. UcsResponseHandler - Handles GRPC response
/// 3. UcsGrpcExecutor - Executes GRPC call
/// 4. UcsFlowExecutor - Executes complete UCS flow using generic ucs_executor
///
/// # Usage
/// ```rust
/// define_ucs_executor! {
///     executor: AuthorizeUcsExecutor,
///     flow: domain::Authorize,
///     request_data: types::PaymentsAuthorizeData,
///     response_data: types::PaymentsResponseData,
///     grpc_request: payments_grpc::PaymentServiceAuthorizeRequest,
///     grpc_response: payments_grpc::PaymentServiceAuthorizeResponse,
///     grpc_method: payment_authorize,
///     response_handler: handle_unified_connector_service_response_for_payment_authorize,
/// }
/// ```
#[macro_export]
macro_rules! define_ucs_executor {
    (
        executor: $executor:ident,
        flow: $flow:ty,
        request_data: $request_data:ty,
        response_data: $response_data:ty,
        grpc_request: $grpc_request:ty,
        grpc_response: $grpc_response:ty,
        grpc_method: $grpc_method:ident,
        response_handler: $response_handler:path,
    ) => {
        /// UCS executor struct
        #[derive(Debug, Clone, Copy)]
        pub struct $executor;

        impl UcsRequestTransformer<$flow, $request_data, $response_data> for $executor {
            type GrpcRequest = $grpc_request;

            fn transform_request(
                router_data: &RouterData<$flow, $request_data, $response_data>,
            ) -> CustomResult<Self::GrpcRequest, ConnectorError> {
                <$grpc_request>::foreign_try_from(router_data)
                    .change_context(ConnectorError::RequestEncodingFailed)
            }
        }

        impl UcsResponseHandler<$grpc_response, $response_data> for $executor {
            fn handle_response(
                response: $grpc_response,
            ) -> CustomResult<
                (
                    Result<($response_data, common_enums::AttemptStatus), ErrorResponse>,
                    u16,
                ),
                ConnectorError,
            > {
                $response_handler(response).change_context(ConnectorError::ResponseHandlingFailed)
            }
        }

        #[async_trait]
        impl UcsGrpcExecutor<
                UnifiedConnectorServiceClient,
                RouterUcsContext,
                $grpc_request,
                $grpc_response,
            > for $executor
        {
            type GrpcResponse = tonic::Response<$grpc_response>;

            async fn execute_grpc_call(
                client: &UnifiedConnectorServiceClient,
                request: $grpc_request,
                context: RouterUcsContext,
            ) -> CustomResult<Self::GrpcResponse, ConnectorError> {
                client
                    .$grpc_method(request, context.auth(), context.headers())
                    .await
                    .change_context(
                        hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError,
                    )
                    .change_context(ConnectorError::ProcessingStepFailed(None))
            }
        }

        #[async_trait]
        impl UcsFlowExecutor<$flow, $request_data, $response_data, SessionState> for $executor {
            type GrpcRequest = $grpc_request;
            type GrpcResponse = $grpc_response;
            type ExecCtx<'a> = RouterUcsExecutionContext<'a>;

            async fn execute_ucs_flow<'a>(
                state: &SessionState,
                router_data: &RouterData<$flow, $request_data, $response_data>,
                execution_context: RouterUcsExecutionContext<'a>,
            ) -> CustomResult<RouterData<$flow, $request_data, $response_data>, ConnectorError>
            where
                Self::GrpcRequest: serde::Serialize + std::fmt::Debug,
                Self::GrpcResponse: std::fmt::Debug,
            {
                ucs_executor::<$flow, $executor, $request_data, $response_data, _, _>(
                    state,
                    router_data,
                    execution_context,
                )
                .await
            }
        }
    };
}

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