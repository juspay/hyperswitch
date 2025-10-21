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
    // pub merchant_context: Option<&'a MerchantContext>,
    pub payment_data: Option<&'a PaymentData>,
    // pub header_payload: Option<&'a HeaderPayload>,
    // // #[cfg(feature = "v2")]
    // pub lineage_ids: Option<LineageIds>,
    pub execution_mode: ExecutionMode,
    pub execution_path: GatewayExecutionPath,
    _phantom: std::marker::PhantomData<F>,
}

impl<'a, F, PaymentData> GatewayExecutionContext<'a, F, PaymentData> {
    /// Create a new gateway execution context
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        // merchant_context: Option<&'a MerchantContext>,
        payment_data: Option<&'a PaymentData>,
        // header_payload: Option<&'a HeaderPayload>,
        // #[cfg(feature = "v2")]
        // lineage_ids: Option<LineageIds>,
        execution_mode: ExecutionMode,
        execution_path: GatewayExecutionPath,
    ) -> Self {
        Self {
            // merchant_context,
            payment_data,
            // header_payload,
            // #[cfg(feature = "v2")]
            // lineage_ids,
            execution_mode,
            execution_path,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create an empty context for backward compatibility
    ///
    /// This is used when calling the gateway abstraction without UCS support.
    /// The execution path defaults to Direct.
    pub fn empty() -> Self {
        Self {
            // merchant_context: None,
            payment_data: None,
            // header_payload: None,
            // #[cfg(feature = "v2")]
            // lineage_ids: None,
            execution_mode: ExecutionMode::Primary,
            execution_path: GatewayExecutionPath::Direct,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Payment gateway trait
///
/// Defines the interface for executing payment operations through different gateway types.
/// Implementations include DirectGateway and UnifiedConnectorServiceGateway.
#[async_trait]
pub trait PaymentGateway<State, ConnectorData, F, Req, Resp, PaymentData>: Send + Sync
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    PaymentData: Clone + Send + Sync + 'static,
{
    /// Execute payment gateway operation
    async fn execute(
        self: Box<Self>,
        state: &State,
        connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
        router_data: &RouterData<F, Req, Resp>,
        call_connector_action: CallConnectorAction,
        connector_request: Option<Request>,
        return_raw_connector_response: Option<bool>,
        context: GatewayExecutionContext<'_, F, PaymentData>,
    ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>;
}

/// Direct gateway implementation
///
/// Executes payment operations through traditional HTTP connector integration.
/// This is the default execution path and maintains backward compatibility.
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
        // Direct gateway delegates to the existing execute_connector_processing_step
        // This maintains backward compatibility with the traditional HTTP-based flow
        api_client::execute_connector_processing_step(
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

pub trait FlowGateway<State, ConnectorData, Req, Resp, PaymentData>:
    Clone + std::fmt::Debug + Send + Sync + 'static
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<Self, Req, Resp> + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    PaymentData: Clone + Send + Sync + 'static,
{
    /// Get the appropriate gateway for this flow based on execution path
    ///
    /// Returns a boxed gateway implementation that can be either:
    /// - DirectGateway for traditional HTTP connector integration
    /// - Flow-specific UCS gateway for gRPC integration
    fn get_gateway(
        execution_path: GatewayExecutionPath,
    ) -> Box<dyn PaymentGateway<State, ConnectorData, Self, Req, Resp, PaymentData>>;
}

/// Factory for creating appropriate gateway instances
///
/// Note: This factory now uses the FlowGateway trait to delegate gateway creation
/// to the flow type itself. Each flow (api::Authorize, api::PSync, etc.) implements
/// FlowGateway to provide its specific gateway based on execution path.
#[derive(Debug, Clone, Copy)]
pub struct GatewayFactory;

impl GatewayFactory {
    pub fn create<State, ConnectorData, F, Req, Resp, PaymentData>(
        execution_path: GatewayExecutionPath,
    ) -> Box<dyn PaymentGateway<State, ConnectorData, F, Req, Resp, PaymentData>>
    where
        State: Clone + Send + Sync + 'static + ApiClientWrapper,
        ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
        F: Clone + std::fmt::Debug + Send + Sync + 'static + FlowGateway<State, ConnectorData, Req, Resp, PaymentData>,
        Req: std::fmt::Debug + Clone + Send + Sync + 'static,
        Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
        PaymentData: Clone + Send + Sync + 'static,
    {
        // Delegate to the flow's FlowGateway implementation
        F::get_gateway(execution_path)
    }
}

impl<State, ConnectorData, Req, Resp> FlowGateway<State, ConnectorData, Req, Resp, ()>
    for DirectGateway
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<Self, Req, Resp> + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
{
    fn get_gateway(
        execution_path: GatewayExecutionPath,
    ) -> Box<
        dyn PaymentGateway<State, ConnectorData, Self, Req, Resp, ()>,
    > {
        match execution_path {
            GatewayExecutionPath::Direct => Box::new(DirectGateway),
            _ => Box::new(DirectGateway), // DirectGateway is the only implementation here
        }
    }
}

/// Execute payment gateway operation (backward compatible version)
///
/// This version maintains backward compatibility by using an empty context.
/// Use `execute_payment_gateway_with_context` for UCS support.
pub async fn execute_payment_gateway<State, ConnectorData, F, Req, Resp, PD>(
    state: &State,
    connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
    router_data: &RouterData<F, Req, Resp>,
    call_connector_action: CallConnectorAction,
    connector_request: Option<Request>,
    return_raw_connector_response: Option<bool>,
    context: Option<GatewayExecutionContext<'_, F, PD>>,
) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static + FlowGateway<State, ConnectorData, Req, Resp, PD>,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    PD: Clone + Send + Sync + 'static,
{
    match context {
        Some(ctx) => {
            // Use provided context
            execute_payment_gateway_with_context::<State, ConnectorData, F, Req, Resp, PD>(
                state,
                connector_integration,
                router_data,
                call_connector_action,
                connector_request,
                return_raw_connector_response,
                ctx,
            )
            .await
        }
        None => {
            // Use empty context for backward compatibility
        api_client::execute_connector_processing_step(
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
}

/// Execute payment gateway operation with context
///
/// This is the main entry point for gateway-based execution with full UCS support.
/// The context determines which gateway implementation to use.
///
/// # Arguments
///
/// * `state` - Application state with API client and configuration
/// * `connector_integration` - Connector-specific integration implementation
/// * `router_data` - Payment operation data and context
/// * `call_connector_action` - Action to perform (Trigger, HandleResponse, etc.)
/// * `connector_request` - Pre-built connector request (optional)
/// * `return_raw_connector_response` - Whether to include raw response in result
/// * `context` - Gateway execution context with UCS information
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
    F: Clone + std::fmt::Debug + Send + Sync + 'static + FlowGateway<State, ConnectorData, Req, Resp, PaymentData>,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    PaymentData: Clone + Send + Sync + 'static,
{
    // Extract execution path from context
    let execution_path = context.execution_path;

    // Create appropriate gateway based on execution path
    // The flow type F implements FlowGateway, which provides the correct gateway
    let gateway: Box<dyn PaymentGateway<State, ConnectorData, F, Req, Resp, PaymentData>> =
    if execution_path == GatewayExecutionPath::Direct {
        // For Direct path, use DirectGateway
        Box::new(DirectGateway)
    } else {
        // For UCS paths, use flow-specific gateway
        F::get_gateway(execution_path)
    };

    // Execute through selected gateway
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