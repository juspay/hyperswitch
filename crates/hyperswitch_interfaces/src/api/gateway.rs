//! Gateway abstraction layer for unified connector execution
//!
//! This module provides a unified interface for executing payment operations through either:
//! - Direct connector integration (traditional HTTP-based)
//! - Unified Connector Service (UCS) via gRPC
//!
//! The gateway abstraction allows seamless switching between execution paths without
//! requiring changes to individual flow implementations.

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionMode};
use common_utils::{errors::CustomResult, request::Request};
use error_stack::{Report, ResultExt};
use crate::{
    api_client::{self, ApiClientWrapper},
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};
use hyperswitch_domain_models::{
    merchant_context::MerchantContext,
    payments::HeaderPayload,
    router_data::RouterData,
};

#[cfg(feature = "v2")]
use external_services::grpc_client::LineageIds;

/// Execution path for gateway operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayExecutionPath {
    /// Direct HTTP connector integration
    Direct,
    /// Unified Connector Service via gRPC
    UnifiedConnectorService,
    /// Shadow mode - execute both paths for comparison
    ShadowUnifiedConnectorService,
}

/// Gateway execution context
///
/// Provides additional context required for UCS gateway execution that is not
/// available in the basic PaymentGateway trait parameters.
///
/// This context is optional to maintain backward compatibility with DirectGateway,
/// which doesn't require this additional information.
///
/// # Type Parameters
/// * `F` - Flow type (e.g., api::Authorize, api::PSync)
/// * `PaymentData` - Payment data type from the operation layer
#[derive(Clone, Debug)]
pub struct GatewayExecutionContext<'a, F, PaymentData> {
    /// Merchant context containing merchant account and key store
    pub merchant_context: Option<&'a MerchantContext>,
    
    /// Payment data from the operation layer
    /// Required for UCS transformations and decision logic
    pub payment_data: Option<&'a PaymentData>,
    
    /// Header payload for gRPC requests
    /// Contains payment confirmation source and other metadata
    pub header_payload: Option<&'a HeaderPayload>,
    
    /// Lineage IDs for distributed tracing
    /// Contains merchant_id and profile_id for request tracking
    #[cfg(feature = "v2")]
    pub lineage_ids: Option<LineageIds>,
    
    /// Execution mode (Primary or Shadow)
    /// Determines whether this is the primary execution or shadow validation
    pub execution_mode: ExecutionMode,
    
    /// Phantom data to maintain type parameter F
    _phantom: std::marker::PhantomData<F>,
}

impl<'a, F, PaymentData> GatewayExecutionContext<'a, F, PaymentData> {
    /// Create a new gateway execution context
    pub fn new(
        merchant_context: Option<&'a MerchantContext>,
        payment_data: Option<&'a PaymentData>,
        header_payload: Option<&'a HeaderPayload>,
        #[cfg(feature = "v2")]
        lineage_ids: Option<LineageIds>,
        execution_mode: ExecutionMode,
    ) -> Self {
        Self {
            merchant_context,
            payment_data,
            header_payload,
            #[cfg(feature = "v2")]
            lineage_ids,
            execution_mode,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Create an empty context for DirectGateway (backward compatibility)
    pub fn empty() -> Self {
        Self {
            merchant_context: None,
            payment_data: None,
            header_payload: None,
            #[cfg(feature = "v2")]
            lineage_ids: None,
            execution_mode: ExecutionMode::Primary,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Core trait for payment gateway operations
///
/// This trait defines the interface for executing payment operations through different
/// gateway implementations (Direct, UCS, etc.)
///
/// # Type Parameters
/// * `State` - Application state type (e.g., SessionState)
/// * `ConnectorData` - Connector data type (e.g., api::ConnectorData)
/// * `F` - Flow type (e.g., api::Authorize, api::PSync)
/// * `Req` - Request data type (e.g., PaymentsAuthorizeData)
/// * `Resp` - Response data type (e.g., PaymentsResponseData)
/// * `PaymentData` - Payment data type from operation layer (for UCS context)
#[async_trait]
pub trait PaymentGateway<State, RouterCommonData, F, Req, Resp, PaymentData = ()>: Send + Sync
where
    State: Clone + Send + Sync + 'static,
    RouterCommonData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    PaymentData: Clone + Send + Sync + 'static,
{
    /// Execute the gateway operation
    ///
    /// This method consumes self to match the ownership requirements of the underlying
    /// connector integration functions.
    ///
    /// # Arguments
    /// * `context` - Optional execution context for UCS gateway (contains MerchantContext, PaymentData, etc.)
    ///               DirectGateway ignores this parameter for backward compatibility.
    async fn execute(
        self: Box<Self>,
        state: &State,
        connector_integration: BoxedConnectorIntegrationInterface<F, RouterCommonData, Req, Resp>,
        router_data: &RouterData<F, Req, Resp>,
        call_connector_action: CallConnectorAction,
        connector_request: Option<Request>,
        return_raw_connector_response: Option<bool>,
        context: GatewayExecutionContext<'_, F, PaymentData>,
    ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>;
}

/// Direct gateway implementation
///
/// Wraps the existing `execute_connector_processing_step` function to provide
/// traditional HTTP-based connector integration.
#[derive(Debug, Clone, Copy)]
pub struct DirectGateway;

#[async_trait]
impl<State, ConnectorData, F, Req, Resp, PaymentData>
    PaymentGateway<State, ConnectorData, F, Req, Resp, PaymentData> for DirectGateway
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    PaymentData: Clone + Send + Sync + 'static,
{
    async fn execute(
        self: Box<Self>,
        state: &State,
        connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
        router_data: &RouterData<F, Req, Resp>,
        call_connector_action: CallConnectorAction,
        connector_request: Option<Request>,
        return_raw_connector_response: Option<bool>,
        _context: GatewayExecutionContext<'_, F, PaymentData>,
    ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError> {
        // DirectGateway ignores the context parameter for backward compatibility
        // It only needs the basic parameters to delegate to execute_connector_processing_step
        
        // Delegate to existing execute_connector_processing_step
        // The type parameters map as follows:
        // - execute_connector_processing_step's T = our F (flow type)
        // - execute_connector_processing_step's ResourceCommonData = our ConnectorData
        api_client::execute_connector_processing_step::<F, ConnectorData, Req, Resp>(
            state,
            connector_integration,
            router_data,
            call_connector_action,
            connector_request,
            return_raw_connector_response,
        )
        .await
    }
}

/// Unified Connector Service gateway implementation
///
/// Handles payment operations through the UCS gRPC service.
/// Currently marked as incomplete - requires additional context for full implementation.
#[derive(Debug, Clone, Copy)]
pub struct UnifiedConnectorServiceGateway;

#[async_trait]
impl<State, ConnectorData, F, Req, Resp, PaymentData>
    PaymentGateway<State, ConnectorData, F, Req, Resp, PaymentData>
    for UnifiedConnectorServiceGateway
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    PaymentData: Clone + Send + Sync + 'static,
{
    async fn execute(
        self: Box<Self>,
        state: &State,
        _connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
        router_data: &RouterData<F, Req, Resp>,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: GatewayExecutionContext<'_, F, PaymentData>,
    ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError> {
        // UCS gateway execution is delegated to flow-specific implementations
        // Each flow (Authorize, PSync, SetupMandate, etc.) has its own UCS call logic
        // in the router crate under core/payments/flows/
        //
        // This gateway serves as the abstraction layer that routes to the appropriate
        // flow-specific UCS implementation based on the flow type F.
        //
        // The context provides all necessary information:
        // - context.merchant_context: MerchantContext for auth metadata
        // - context.payment_data: PaymentData for request transformations
        // - context.header_payload: HeaderPayload for gRPC headers
        // - context.lineage_ids: LineageIds for distributed tracing (v2)
        // - context.execution_mode: ExecutionMode (Primary vs Shadow)
        //
        // Implementation approach:
        // Since each flow has unique request/response types and UCS gRPC methods,
        // the actual UCS calls are implemented in flow-specific trait methods
        // (e.g., Feature::call_unified_connector_service in psync_flow.rs)
        //
        // This gateway implementation returns an error indicating that UCS execution
        // should be handled by the flow-specific code path, not through the generic
        // gateway abstraction.
        
        Err(Report::new(ConnectorError::NotImplemented(
            "UCS execution is handled by flow-specific implementations. \
             Use Feature::call_unified_connector_service instead of gateway abstraction."
                .to_string(),
        ))
        .attach_printable("Attempted to execute UCS gateway through generic abstraction")
    )
    }
}

/// Factory for creating appropriate gateway instances
#[derive(Debug, Clone, Copy)]
pub struct GatewayFactory;

impl GatewayFactory {
    /// Create a gateway instance based on execution path
    ///
    /// Returns the appropriate gateway implementation based on the execution path:
    /// - Direct: Traditional HTTP connector integration
    /// - UnifiedConnectorService: UCS gRPC integration (when implemented)
    /// - ShadowUnifiedConnectorService: Both paths for validation (when implemented)
    ///
    /// # Type Parameters
    /// * `PaymentData` - Payment data type from operation layer (default: ())
    pub fn create<State, ConnectorData, F, Req, Resp, PaymentData>(
        execution_path: GatewayExecutionPath,
    ) -> Box<dyn PaymentGateway<State, ConnectorData, F, Req, Resp, PaymentData>>
    where
        State: Clone + Send + Sync + 'static + ApiClientWrapper,
        ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
        F: Clone + std::fmt::Debug + Send + Sync + 'static,
        Req: std::fmt::Debug + Clone + Send + Sync + 'static,
        Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
        PaymentData: Clone + Send + Sync + 'static,
    {
        match execution_path {
            GatewayExecutionPath::Direct => Box::new(DirectGateway),
            
            GatewayExecutionPath::UnifiedConnectorService => {
                // TODO: Return UCS gateway when implementation is complete
                // For now, fall back to Direct gateway
                Box::new(DirectGateway)
            }
            
            GatewayExecutionPath::ShadowUnifiedConnectorService => {
                // TODO: Return Shadow gateway when implementation is complete
                // For now, fall back to Direct gateway
                Box::new(DirectGateway)
            }
        }
    }
}

/// Execute payment gateway operation
///
/// This is the main entry point for all payment operations. It replaces direct calls to
/// `execute_connector_processing_step` and provides a unified interface that can route
/// to either Direct or UCS execution paths.
///
/// # Arguments
///
/// * `state` - Application state with API client and configuration
/// * `connector_integration` - Connector-specific integration implementation
/// * `router_data` - Payment operation data and context
/// * `call_connector_action` - Action to perform (Trigger, HandleResponse, etc.)
/// * `connector_request` - Pre-built connector request (optional)
/// * `return_raw_connector_response` - Whether to include raw response in result
///
/// # Returns
///
/// Updated RouterData with response from the gateway execution
/// Execute payment gateway operation (backward compatible version)
///
/// This version maintains backward compatibility by using an empty context.
/// Use `execute_payment_gateway_with_context` for UCS support.
pub async fn execute_payment_gateway<State, ConnectorData, F, Req, Resp>(
    state: &State,
    connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
    router_data: &RouterData<F, Req, Resp>,
    call_connector_action: CallConnectorAction,
    connector_request: Option<Request>,
    return_raw_connector_response: Option<bool>,
) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
{
    // Use empty context for backward compatibility
    execute_payment_gateway_with_context::<State, ConnectorData, F, Req, Resp, ()>(
        state,
        connector_integration,
        router_data,
        call_connector_action,
        connector_request,
        return_raw_connector_response,
        GatewayExecutionContext::empty(),
    )
    .await
}

/// Execute payment gateway operation with execution context
///
/// This version supports UCS gateway by accepting a GatewayExecutionContext parameter.
/// The context provides MerchantContext, PaymentData, HeaderPayload, and LineageIds
/// required for UCS gRPC calls.
///
/// # Arguments
///
/// * `state` - Application state with API client and configuration
/// * `connector_integration` - Connector-specific integration implementation
/// * `router_data` - Payment operation data and context
/// * `call_connector_action` - Action to perform (Trigger, HandleResponse, etc.)
/// * `connector_request` - Pre-built connector request (optional)
/// * `return_raw_connector_response` - Whether to include raw response in result
/// * `context` - Gateway execution context for UCS support
///
/// # Returns
///
/// Updated RouterData with response from the gateway execution
pub async fn execute_payment_gateway_with_context<State, ConnectorData, F, Req, Resp, PaymentData>(
    state: &State,
    connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
    router_data: &RouterData<F, Req, Resp>,
    call_connector_action: CallConnectorAction,
    connector_request: Option<Request>,
    return_raw_connector_response: Option<bool>,
    context: GatewayExecutionContext<'_, F, PaymentData>,
) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    PaymentData: Clone + Send + Sync + 'static,
{
    // Determine execution path
    // For now, always use Direct path until UCS implementation is complete
    // TODO: Use context.merchant_context to call should_call_unified_connector_service()
    let execution_path = GatewayExecutionPath::Direct;

    // Create appropriate gateway
    let gateway: Box<dyn PaymentGateway<State, ConnectorData, F, Req, Resp, PaymentData>> =
        GatewayFactory::create(execution_path);

    // Execute through gateway with context
    gateway
        .execute(
            state,
            connector_integration,
            router_data,
            call_connector_action,
            connector_request,
            return_raw_connector_response,
            context,
        )
        .await
        .attach_printable("Gateway execution failed")
}