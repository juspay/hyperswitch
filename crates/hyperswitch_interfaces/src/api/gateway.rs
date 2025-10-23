//! Gateway abstraction layer for unified connector execution
//!
//! This module provides a unified interface for executing payment operations through either:
//! - Direct connector integration (traditional HTTP-based)
//! - Unified Connector Service (UCS) via gRPC
//!
//! The gateway abstraction allows seamless switching between execution paths without
//! requiring changes to individual flow implementations.

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionMode, ExecutionPath};
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

/// Minimal trait that gateway context must implement
///
/// This allows the framework to extract execution metadata without knowing
/// the concrete context structure. Implementation crates define their own
/// context types with whatever fields they need.
pub trait GatewayContext: Clone + Send + Sync {
    /// Get the execution path (Direct, UCS, or Shadow)
    fn execution_path(&self) -> ExecutionPath;
    
    /// Get the execution mode (Primary, Shadow, etc.)
    fn execution_mode(&self) -> ExecutionMode;
}



/// Payment gateway trait
///
/// Defines the interface for executing payment operations through different gateway types.
/// Implementations include DirectGateway and flow-specific UCS gateways.
///
/// # Type Parameters
/// * `State` - Application state (e.g., SessionState)
/// * `ConnectorData` - Connector-specific data type
/// * `F` - Flow type (e.g., domain::Authorize, domain::PSync)
/// * `Req` - Request data type
/// * `Resp` - Response data type
/// * `Context` - Gateway context type (must implement GatewayContext trait)
#[async_trait]
pub trait PaymentGateway<State, ConnectorData, F, Req, Resp, Context>: Send + Sync
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    Context: GatewayContext,
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
        context: Context,
    ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>;
}

// /// Direct gateway implementation
// ///
// /// Executes payment operations through traditional HTTP connector integration.
// /// This is the default execution path and maintains backward compatibility.
// #[derive(Debug, Clone, Copy)]
// pub struct DirectGateway;

// #[async_trait]
// impl<State, ConnectorData, F, Req, Resp, Context>
//     PaymentGateway<State, ConnectorData, F, Req, Resp, Context> for DirectGateway
// where
//     State: Clone + Send + Sync + 'static + ApiClientWrapper,
//     ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
//     F: Clone + std::fmt::Debug + Send + Sync + 'static,
//     Req: std::fmt::Debug + Clone + Send + Sync + 'static,
//     Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
//     Context: GatewayContext + 'static,
// {
//     async fn execute(
//         self: Box<Self>,
//         state: &State,
//         connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
//         router_data: &RouterData<F, Req, Resp>,
//         call_connector_action: CallConnectorAction,
//         connector_request: Option<Request>,
//         return_raw_connector_response: Option<bool>,
//         _context: Context,
//     ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError> {
//         // Direct gateway delegates to the existing execute_connector_processing_step
//         // This maintains backward compatibility with the traditional HTTP-based flow
//         api_client::execute_connector_processing_step(
//             state,
//             connector_integration,
//             router_data,
//             call_connector_action,
//             connector_request,
//             return_raw_connector_response,
//         )
//         .await
//     }
// }

pub trait FlowGateway<State, ConnectorData, Req, Resp, Context>:
    Clone + std::fmt::Debug + Send + Sync + 'static
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<Self, Req, Resp> + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    Context: GatewayContext,
{
    /// Get the appropriate gateway for this flow based on execution path
    ///
    /// Returns a boxed gateway implementation that can be either:
    /// - DirectGateway for traditional HTTP connector integration
    /// - Flow-specific UCS gateway for gRPC integration
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<dyn PaymentGateway<State, ConnectorData, Self, Req, Resp, Context>>;
}

/// Factory for creating appropriate gateway instances
///
/// Note: This factory now uses the FlowGateway trait to delegate gateway creation
/// to the flow type itself. Each flow (api::Authorize, api::PSync, etc.) implements
/// FlowGateway to provide its specific gateway based on execution path.
#[derive(Debug, Clone, Copy)]
pub struct GatewayFactory;

impl GatewayFactory {
    pub fn create<State, ConnectorData, F, Req, Resp, Context>(
        execution_path: ExecutionPath,
    ) -> Box<dyn PaymentGateway<State, ConnectorData, F, Req, Resp, Context>>
    where
        State: Clone + Send + Sync + 'static + ApiClientWrapper,
        ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
        F: Clone + std::fmt::Debug + Send + Sync + 'static + FlowGateway<State, ConnectorData, Req, Resp, Context>,
        Req: std::fmt::Debug + Clone + Send + Sync + 'static,
        Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
        Context: GatewayContext,
    {
        // Delegate to the flow's FlowGateway implementation
        F::get_gateway(execution_path)
    }
}

/// Empty context for DirectGateway (backward compatibility)
#[derive(Debug, Clone, Copy)]
pub struct EmptyContext;

impl GatewayContext for EmptyContext {
    fn execution_path(&self) -> ExecutionPath {
        ExecutionPath::Direct
    }
    
    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Primary
    }
}

// impl<State, ConnectorData, Req, Resp> FlowGateway<State, ConnectorData, Req, Resp, EmptyContext>
//     for DirectGateway
// where
//     State: Clone + Send + Sync + 'static + ApiClientWrapper,
//     ConnectorData: Clone + RouterDataConversion<Self, Req, Resp> + Send + Sync + 'static,
//     Req: std::fmt::Debug + Clone + Send + Sync + 'static,
//     Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
// {
//     fn get_gateway(
//         execution_path: ExecutionPath,
//     ) -> Box<
//         dyn PaymentGateway<State, ConnectorData, Self, Req, Resp, EmptyContext>,
//     > {
//         match execution_path {
//             ExecutionPath::Direct => Box::new(DirectGateway),
//             _ => Box::new(DirectGateway), // DirectGateway is the only implementation here
//         }
//     }
// }

/// Execute payment gateway operation (backward compatible version)
///
/// This version maintains backward compatibility by using direct execution when no context is provided.
/// Use `execute_payment_gateway_with_context` for UCS support.
pub async fn execute_payment_gateway<State, ConnectorData, F, Req, Resp, Context>(
    state: &State,
    connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
    router_data: &RouterData<F, Req, Resp>,
    call_connector_action: CallConnectorAction,
    connector_request: Option<Request>,
    return_raw_connector_response: Option<bool>,
    context: Option<Context>,
) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static + FlowGateway<State, ConnectorData, Req, Resp, Context>,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    Context: GatewayContext,
{
    match context {
        Some(ctx) => {
            // Use provided context
            execute_payment_gateway_with_context::<State, ConnectorData, F, Req, Resp, Context>(
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
            // Use direct execution for backward compatibility
            api_client::execute_connector_processing_step(
                state,
                connector_integration,
                router_data,
                call_connector_action,
                connector_request,
                return_raw_connector_response,
            )
            .await
            .to_payment_failed_response()
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
/// * `context` - Gateway execution context (implementation-defined type)
///
/// # Returns
///
/// Updated RouterData with response from the gateway execution
pub async fn execute_payment_gateway_with_context<State, ConnectorData, F, Req, Resp, Context>(
    state: &State,
    connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
    router_data: &RouterData<F, Req, Resp>,
    call_connector_action: CallConnectorAction,
    connector_request: Option<Request>,
    return_raw_connector_response: Option<bool>,
    context: Context,
) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static + FlowGateway<State, ConnectorData, Req, Resp, Context>,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    Context: GatewayContext,
{
    // Extract execution path from context
    let execution_path = context.execution_path();

    // Create appropriate gateway based on execution path
    // The flow type F implements FlowGateway, which provides the correct gateway
    let gateway: Box<dyn PaymentGateway<State, ConnectorData, F, Req, Resp, Context>> =
        F::get_gateway(execution_path);

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