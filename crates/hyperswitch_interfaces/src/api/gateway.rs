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
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data::RouterData;
use router_env::logger;

use crate::{
    api_client::{self, ApiClientWrapper},
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
    helpers,
};

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
#[allow(clippy::too_many_arguments)]
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

/// Direct gateway implementation
///
/// Executes payment operations through traditional HTTP connector integration.
/// This is the default execution path and maintains backward compatibility.
#[derive(Debug, Clone, Copy)]
pub struct DirectGateway;

#[async_trait]
impl<State, ConnectorData, F, Req, Resp, Context>
    PaymentGateway<State, ConnectorData, F, Req, Resp, Context> for DirectGateway
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    Context: GatewayContext + 'static,
{
    async fn execute(
        self: Box<Self>,
        state: &State,
        connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
        router_data: &RouterData<F, Req, Resp>,
        call_connector_action: CallConnectorAction,
        connector_request: Option<Request>,
        return_raw_connector_response: Option<bool>,
        _context: Context,
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

/// Flow gateway trait for determining execution path
///
/// This trait allows flows to specify which gateway implementation should be used
/// based on the execution path. Each flow implements this trait to provide
/// flow-specific gateway selection logic.
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
    context: Context,
) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper + helpers::GetComparisonServiceConfig,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone
        + std::fmt::Debug
        + Send
        + Sync
        + 'static
        + FlowGateway<State, ConnectorData, Req, Resp, Context>,
    Req: std::fmt::Debug + Clone + Send + Sync + serde::Serialize + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + serde::Serialize + 'static,
    Context: GatewayContext + 'static,
{
    let execution_path = context.execution_path();

    match execution_path {
        ExecutionPath::Direct => {
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
        }
        ExecutionPath::UnifiedConnectorService => {
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
        ExecutionPath::ShadowUnifiedConnectorService => {
            let direct_router_data = api_client::execute_connector_processing_step(
                state,
                connector_integration.clone_box(),
                router_data,
                call_connector_action.clone(),
                connector_request,
                return_raw_connector_response,
            )
            .await?;
            let state_clone = state.clone();
            let router_data_clone = router_data.clone();
            let direct_router_data_clone = direct_router_data.clone();
            let return_raw_connector_response_clone = return_raw_connector_response;
            let context_clone = context.clone();
            tokio::spawn(async move {
                let gateway: Box<dyn PaymentGateway<State, ConnectorData, F, Req, Resp, Context>> =
                    F::get_gateway(execution_path);
                let ucs_shadow_result = gateway
                    .execute(
                        &state_clone,
                        connector_integration,
                        &router_data_clone,
                        call_connector_action,
                        None,
                        return_raw_connector_response_clone,
                        context_clone,
                    )
                    .await
                    .attach_printable("Gateway execution failed");
                // Send comparison data asynchronously
                match ucs_shadow_result {
                    Ok(ucs_router_data) => {
                        // Send comparison data asynchronously
                        if let Some(comparison_service_config) =
                            state_clone.get_comparison_service_config()
                        {
                            let request_id = state_clone.get_request_id_str();
                            let _ = helpers::serialize_router_data_and_send_to_comparison_service(
                                &state_clone,
                                direct_router_data_clone,
                                ucs_router_data,
                                comparison_service_config,
                                request_id,
                            )
                            .await;
                        };
                    }
                    Err(e) => {
                        logger::error!("UCS shadow execution failed: {:?}", e);
                    }
                }
            });
            Ok(direct_router_data)
        }
    }
}
