//! Gateway abstraction layer for unified connector execution
//!
//! This module provides a unified interface for executing payment operations through either:
//! - Direct connector integration (traditional HTTP-based)
//! - Unified Connector Service (UCS) via gRPC
//!
//! The gateway abstraction allows seamless switching between execution paths without
//! requiring changes to individual flow implementations.

use async_trait::async_trait;
use common_enums::CallConnectorAction;
use common_utils::{errors::CustomResult, request::Request};
use error_stack::ResultExt;
use crate::{
    api_client::{self, ApiClientWrapper},
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};
use hyperswitch_domain_models::router_data::RouterData;

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
#[async_trait]
pub trait PaymentGateway<State, RouterCommonData, F, Req, Resp>: Send + Sync
where
    State: Clone + Send + Sync + 'static,
    RouterCommonData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
{
    /// Execute the gateway operation
    ///
    /// This method consumes self to match the ownership requirements of the underlying
    /// connector integration functions.
    async fn execute(
        self: Box<Self>,
        state: &State,
        connector_integration: BoxedConnectorIntegrationInterface<F, RouterCommonData, Req, Resp>,
        router_data: &RouterData<F, Req, Resp>,
        call_connector_action: CallConnectorAction,
        connector_request: Option<Request>,
        return_raw_connector_response: Option<bool>,
    ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>;
}

/// Direct gateway implementation
///
/// Wraps the existing `execute_connector_processing_step` function to provide
/// traditional HTTP-based connector integration.
pub struct DirectGateway;

#[async_trait]
impl<State, ConnectorData, F, Req, Resp>
    PaymentGateway<State, ConnectorData, F, Req, Resp> for DirectGateway
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
{
    async fn execute(
        self: Box<Self>,
        state: &State,
        connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
        router_data: &RouterData<F, Req, Resp>,
        call_connector_action: CallConnectorAction,
        connector_request: Option<Request>,
        return_raw_connector_response: Option<bool>,
    ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError> {
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
pub struct UnifiedConnectorServiceGateway;

#[async_trait]
impl<State, ConnectorData, F, Req, Resp>
    PaymentGateway<State, ConnectorData, F, Req, Resp>
    for UnifiedConnectorServiceGateway
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
{
    async fn execute(
        self: Box<Self>,
        _state: &State,
        _connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
        _router_data: &RouterData<F, Req, Resp>,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
    ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError> {
        // TODO: Implement UCS gateway execution
        // This requires additional context that is not available at this layer:
        // - MerchantContext
        // - PaymentData
        // - HeaderPayload
        // - LineageIds
        //
        // These will need to be passed from the higher-level payment flow orchestration
        todo!("UCS gateway implementation pending - requires additional context from payment flow layer")
    }
}

/// Factory for creating appropriate gateway instances
pub struct GatewayFactory;

impl GatewayFactory {
    /// Create a gateway instance based on execution path
    ///
    /// Currently always returns Direct gateway as UCS support is incomplete.
    /// This will be enhanced to support dynamic path selection based on:
    /// - Connector capabilities
    /// - Merchant configuration
    /// - Feature flags
    /// - Rollout percentage
    pub fn create<State, ConnectorData, F, Req, Resp>(
        _execution_path: GatewayExecutionPath,
    ) -> Box<dyn PaymentGateway<State, ConnectorData, F, Req, Resp>>
    where
        State: Clone + Send + Sync + 'static + ApiClientWrapper,
        ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
        F: Clone + std::fmt::Debug + Send + Sync + 'static,
        Req: std::fmt::Debug + Clone + Send + Sync + 'static,
        Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    {
        // Always return Direct gateway for now
        // TODO: Implement path selection logic when UCS is ready
        Box::new(DirectGateway)
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
    // Determine execution path
    // For now, always use Direct path until UCS implementation is complete
    let execution_path = GatewayExecutionPath::Direct;

    // Create appropriate gateway
    let gateway: Box<dyn PaymentGateway<State, ConnectorData, F, Req, Resp>> =
        GatewayFactory::create(execution_path);

    // Execute through gateway
    gateway
        .execute(
            state,
            connector_integration,
            router_data,
            call_connector_action,
            connector_request,
            return_raw_connector_response,
        )
        .await
        .attach_printable("Gateway execution failed")
}