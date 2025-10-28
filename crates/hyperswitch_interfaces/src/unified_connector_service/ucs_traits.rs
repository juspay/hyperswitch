//! UCS (Unified Connector Service) Trait-Based Architecture
//!
//! This module defines traits for implementing GRPC-based payment flows through UCS.
//! The trait-based approach eliminates code duplication across different payment flows
//! by separating common infrastructure from polymorphic operations.
//!
//! ## Architecture Overview
//!
//! Each UCS flow implementation requires three polymorphic operations:
//! 1. **Request Transformation** - Convert RouterData to GRPC request
//! 2. **GRPC Execution** - Call the appropriate GRPC client method
//! 3. **Response Handling** - Convert GRPC response back to RouterData
//!
//! Common infrastructure (client initialization, auth metadata, headers) is handled
//! by the orchestrator trait `UcsFlowExecutor`.
//!
//! ## Generic Associated Types (GATs)
//!
//! This module uses GATs to avoid depending on `external_services` crate.
//! Consumer crates (like `router`) provide their own concrete types through:
//! - **UcsContext**: Groups AuthMetadata, GrpcHeaders, LineageIds
//! - **GrpcClient**: The GRPC client type
//! - **EventHandler**: The event handler for logging
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! // Define an executor for a new GRPC endpoint
//! pub struct CaptureUcsExecutor;
//!
//! impl UcsRequestTransformer<Capture, PaymentsCaptureData, PaymentsResponseData>
//!     for CaptureUcsExecutor {
//!     type GrpcRequest = PaymentServiceCaptureRequest;
//!     
//!     fn transform_request(
//!         router_data: &RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>
//!     ) -> CustomResult<Self::GrpcRequest, ConnectorError> {
//!         PaymentServiceCaptureRequest::foreign_try_from(router_data)
//!             .change_context(ConnectorError::RequestEncodingFailed)
//!     }
//! }
//!
//! impl UcsResponseHandler<PaymentServiceCaptureResponse> for CaptureUcsExecutor {
//!     fn handle_response(
//!         response: PaymentServiceCaptureResponse
//!     ) -> CustomResult<(Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>, u16), ConnectorError> {
//!         handle_unified_connector_service_response_for_payment_capture(response)
//!             .change_context(ConnectorError::ResponseHandlingFailed)
//!     }
//! }
//!
//! #[async_trait]
//! impl<Client, Ctx> UcsGrpcExecutor<Client, Ctx, PaymentServiceCaptureRequest, PaymentServiceCaptureResponse>
//!     for CaptureUcsExecutor
//! where
//!     Ctx: UcsContext,
//! {
//!     type GrpcResponse = tonic::Response<PaymentServiceCaptureResponse>;
//!
//!     async fn execute_grpc_call(
//!         client: &Client,
//!         request: PaymentServiceCaptureRequest,
//!         context: Ctx,
//!     ) -> CustomResult<Self::GrpcResponse, ConnectorError> {
//!         client.payment_capture(request, context.auth(), context.headers())
//!             .await
//!             .change_context(ConnectorError::ProcessingStepFailed(None))
//!     }
//! }
//!
//! // UcsFlowExecutor is automatically implemented!
//! // Now you can call: CaptureUcsExecutor::execute_ucs_flow(state, router_data, exec_context)
//! ```

use async_trait::async_trait;
use common_enums::AttemptStatus;
use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{router_data::ErrorResponse, router_response_types::PaymentsResponseData};

use crate::errors::ConnectorError;

/// Trait representing the context needed for UCS GRPC calls
///
/// This trait groups together the authentication metadata, GRPC headers,
/// and lineage IDs that are needed for making GRPC calls to UCS.
///
/// Consumer crates implement this trait to provide their own concrete types.
pub trait UcsContext {
    /// The authentication metadata type
    type AuthMetadata;
    
    /// The GRPC headers type
    type GrpcHeaders;
    
    /// The lineage IDs type
    type LineageIds;

    /// Get the authentication metadata
    fn auth(&self) -> Self::AuthMetadata;
    
    /// Get the GRPC headers
    fn headers(self) -> Self::GrpcHeaders;
    
    /// Get the lineage IDs
    fn lineage_ids(&self) -> &Self::LineageIds;
}

/// Trait for transforming RouterData into GRPC request types
///
/// This trait handles the polymorphic step of converting domain-specific RouterData
/// into the appropriate GRPC request type for each payment flow.
///
/// # Type Parameters
/// - `F`: Flow type (e.g., Authorize, Capture, PSync)
/// - `Req`: Request data type (e.g., PaymentsAuthorizeData)
/// - `Resp`: Response data type (e.g., PaymentsResponseData)
pub trait UcsRequestTransformer<F, Req, Resp> {
    /// The GRPC request type this transformer produces
    type GrpcRequest;

    /// Transform RouterData into a GRPC request
    ///
    /// # Arguments
    /// - `router_data`: The RouterData containing request information
    ///
    /// # Returns
    /// The GRPC request ready to be sent to UCS
    fn transform_request(
        router_data: &hyperswitch_domain_models::router_data::RouterData<F, Req, Resp>,
    ) -> CustomResult<Self::GrpcRequest, ConnectorError>;
}

/// Trait for handling GRPC responses and converting them to RouterData format
///
/// This trait handles the polymorphic step of processing GRPC responses and
/// converting them into the standard RouterData response format.
///
/// # Type Parameters
/// - `GrpcResp`: The GRPC response type to handle
/// - `Resp`: The response data type (e.g., PaymentsResponseData)
pub trait UcsResponseHandler<GrpcResp, Resp> {
    /// Handle a GRPC response and convert it to RouterData format
    ///
    /// # Arguments
    /// - `response`: The GRPC response from UCS
    ///
    /// # Returns
    /// A tuple containing:
    /// - Result with response data and AttemptStatus, or ErrorResponse
    /// - HTTP status code
    fn handle_response(
        response: GrpcResp,
    ) -> CustomResult<
        (
            Result<(Resp, AttemptStatus), ErrorResponse>,
            u16,
        ),
        ConnectorError,
    >;
}

/// Trait for executing GRPC client method calls
///
/// This trait handles the polymorphic step of calling the appropriate GRPC
/// client method for each payment flow.
///
/// Uses Generic Associated Types (GATs) to allow consumer crates to provide
/// their own GRPC client and context types.
///
/// # Type Parameters
/// - `Client`: The GRPC client type (e.g., UnifiedConnectorServiceClient)
/// - `Ctx`: The UCS context type containing auth, headers, and lineage IDs
/// - `GrpcReq`: The GRPC request type
/// - `GrpcResp`: The GRPC response type (inner response, not wrapped)
#[async_trait]
pub trait UcsGrpcExecutor<Client, Ctx, GrpcReq, GrpcResp>
where
    Ctx: UcsContext,
{
    /// The GRPC response wrapper type (e.g., tonic::Response<GrpcResp>)
    type GrpcResponse;

    /// Execute a GRPC call to UCS
    ///
    /// # Arguments
    /// - `client`: The UCS GRPC client
    /// - `request`: The GRPC request to send
    /// - `context`: The UCS context containing auth, headers, and lineage IDs
    ///
    /// # Returns
    /// The GRPC response (wrapped in framework-specific response type)
    async fn execute_grpc_call(
        client: &Client,
        request: GrpcReq,
        context: Ctx,
    ) -> CustomResult<Self::GrpcResponse, ConnectorError>;
}

/// Execution context for UCS flows
///
/// This struct groups all the context parameters needed for UCS execution,
/// making the API cleaner and more maintainable.
pub struct UcsExecutionContext<MerchantCtx, HeaderPayload, LineageIds, MerchantConnectorAccount> {
    pub merchant_context: MerchantCtx,
    pub header_payload: HeaderPayload,
    pub lineage_ids: LineageIds,
    pub merchant_connector_account: MerchantConnectorAccount,
    pub execution_mode: common_enums::ExecutionMode,
}

/// Main orchestrator trait that combines all UCS operations
///
/// This trait is automatically implemented for any type that implements
/// UcsRequestTransformer, UcsResponseHandler, and UcsGrpcExecutor.
///
/// It provides the complete UCS flow execution including:
/// 1. Common infrastructure setup (client, auth, headers)
/// 2. Request transformation
/// 3. GRPC call execution with logging
/// 4. Response handling
/// 5. RouterData update
///
/// Uses Generic Associated Types (GATs) to allow consumer crates to provide
/// their own types for GRPC infrastructure.
///
/// # Type Parameters
/// - `F`: Flow type (e.g., Authorize, Capture, PSync)
/// - `Req`: Request data type (e.g., PaymentsAuthorizeData)
/// - `Resp`: Response data type (e.g., PaymentsResponseData)
#[async_trait]
pub trait UcsFlowExecutor<F, Req, Resp, State>: Send + Sync {
    /// The GRPC request type for this flow
    type GrpcRequest: Send + Sync;
    
    /// The GRPC response type for this flow
    type GrpcResponse: Send + Sync + Clone;

    type ExecCtx<'a>: UcsExecutionContextProvider;

    /// Execute the complete UCS flow
    ///
    /// This method orchestrates the entire UCS execution:
    /// 1. Prepares common infrastructure (client, auth, headers) from execution context
    /// 2. Transforms the request using `transform_request`
    /// 3. Executes the GRPC call using `execute_grpc_call` wrapped in logging
    /// 4. Handles the response using `handle_response`
    /// 5. Updates and returns the RouterData
    ///
    /// # Arguments
    /// - `state`: Session state containing GRPC client and event handler
    /// - `router_data`: The RouterData to process
    /// - `execution_context`: Context containing merchant info, headers, lineage IDs, etc.
    ///
    /// # Returns
    /// Updated RouterData with response from UCS
    async fn execute_ucs_flow<'a>(
        state: &State,
        router_data: &hyperswitch_domain_models::router_data::RouterData<F, Req, Resp>,
        execution_context: Self::ExecCtx<'a>,
    ) -> CustomResult<
        hyperswitch_domain_models::router_data::RouterData<F, Req, Resp>,
        ConnectorError,
    >
    where
        Self::GrpcRequest: serde::Serialize + std::fmt::Debug,
        Self::GrpcResponse: std::fmt::Debug;
}

/// Trait for providing execution context
///
/// This trait allows different implementations to provide the execution context
/// in different ways (e.g., from a struct, from state, etc.)
pub trait UcsExecutionContextProvider {
    type MerchantContext;
    type HeaderPayload;
    type LineageIds;
    type MerchantConnectorAccount;

    fn merchant_context(&self) -> &Self::MerchantContext;
    fn header_payload(&self) -> &Self::HeaderPayload;
    fn lineage_ids(&self) -> Self::LineageIds;
    fn merchant_connector_account(&self) -> &Self::MerchantConnectorAccount;
    fn execution_mode(&self) -> common_enums::ExecutionMode;
}

/// Trait for providing UCS-related state and utilities
///
/// This trait abstracts the state provider (e.g., SessionState) to allow
/// the UCS traits to work with different state implementations.
///
/// Uses Generic Associated Types (GATs) to allow consumer crates to provide
/// their own types for GRPC infrastructure.
pub trait UcsStateProvider {
    /// The GRPC client type
    type GrpcClient;
    
    /// The UCS context type (contains auth, headers, lineage IDs)
    type Context: UcsContext;
    
    /// The context builder type for constructing UCS context
    type ContextBuilder;
    
    /// The lineage IDs type
    type LineageIds;

    /// Get the UCS GRPC client
    fn get_ucs_client(&self) -> CustomResult<&Self::GrpcClient, ConnectorError>;
    
    /// Get context builder for UCS
    fn get_context_builder(
        &self,
        execution_mode: common_enums::ExecutionMode,
    ) -> Self::ContextBuilder;
}