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
    pub merchant_context: Option<&'a MerchantContext>,
    pub payment_data: Option<&'a PaymentData>,
    pub header_payload: Option<&'a HeaderPayload>,
    #[cfg(feature = "v2")]
    pub lineage_ids: Option<LineageIds>,
    pub execution_mode: ExecutionMode,
    pub execution_path: GatewayExecutionPath,
    _phantom: std::marker::PhantomData<F>,
}

impl<'a, F, PaymentData> GatewayExecutionContext<'a, F, PaymentData> {
    /// Create a new gateway execution context
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        merchant_context: Option<&'a MerchantContext>,
        payment_data: Option<&'a PaymentData>,
        header_payload: Option<&'a HeaderPayload>,
        #[cfg(feature = "v2")]
        lineage_ids: Option<LineageIds>,
        execution_mode: ExecutionMode,
        execution_path: GatewayExecutionPath,
    ) -> Self {
        Self {
            merchant_context,
            payment_data,
            header_payload,
            #[cfg(feature = "v2")]
            lineage_ids,
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
            merchant_context: None,
            payment_data: None,
            header_payload: None,
            #[cfg(feature = "v2")]
            lineage_ids: None,
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

/// Unified Connector Service gateway implementation
///
/// Handles payment operations through the UCS gRPC service by delegating to
/// flow-specific implementations in the router crate.
///
/// # Architecture
///
/// This gateway serves as a routing layer that delegates UCS execution to flow-specific
/// implementations. Each payment flow (Authorize, PSync, SetupMandate, etc.) has its own
/// UCS logic because:
/// - Different flows use different gRPC methods (payment_authorize, payment_get, etc.)
/// - Request/response transformations are flow-specific
/// - Each flow has unique business logic requirements
///
/// # Execution Flow
///
/// 1. Router layer calls `execute_payment_gateway_with_context()`
/// 2. GatewayFactory creates UnifiedConnectorServiceGateway based on execution_path
/// 3. UnifiedConnectorServiceGateway.execute() is called
/// 4. Returns NotImplemented error to signal router to use flow-specific UCS path
/// 5. Router falls back to calling Feature::call_unified_connector_service()
///
/// # Context Usage
///
/// The context provides all necessary information for UCS execution:
/// - `merchant_context`: MerchantContext for building auth metadata
/// - `payment_data`: PaymentData for request transformations
/// - `header_payload`: HeaderPayload for gRPC headers
/// - `lineage_ids`: LineageIds for distributed tracing (v2 only)
/// - `execution_mode`: ExecutionMode (Primary vs Shadow)
///
/// # Why Not Generic Implementation?
///
/// A generic UCS implementation is not feasible because:
/// - Flow type `F` is a generic parameter - cannot pattern match at runtime
/// - Each flow requires different gRPC client methods
/// - Request/response types vary by flow
/// - Business logic differs significantly between flows
///
/// # Future Improvements
///
/// Potential approaches for more generic UCS handling:
/// - Trait-based dispatch with flow-specific UCS traits
/// - Macro-generated implementations for common patterns
/// - Type-level flow identification for compile-time dispatch
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
        _state: &State,
        _connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
        router_data: &RouterData<F, Req, Resp>,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        _context: GatewayExecutionContext<'_, F, PaymentData>,
    ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError> {
        // Return NotImplemented to signal that UCS execution should be handled
        // by flow-specific implementations in the router crate.
        //
        // This is intentional - the gateway abstraction serves as a routing layer,
        // and the actual UCS logic lives in Feature::call_unified_connector_service
        // implementations for each flow type.
        //
        // The router layer will catch this error and fall back to calling the
        // flow-specific UCS implementation with the full context.
        Err(Report::new(ConnectorError::NotImplemented(
            format!(
                "UCS execution for flow '{}' is delegated to flow-specific implementation. \
                 Router will call Feature::call_unified_connector_service.",
                std::any::type_name::<F>()
            ),
        ))
        .attach_printable("UnifiedConnectorServiceGateway delegates to flow-specific UCS logic")
        .attach_printable(format!("Flow type: {}", std::any::type_name::<F>()))
        .attach_printable(format!("Connector: {}", router_data.connector))
        .attach_printable(format!("Payment ID: {}", router_data.payment_id)))
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
    /// - UnifiedConnectorService: UCS gRPC integration
    /// - ShadowUnifiedConnectorService: Both paths for validation (future)
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
                Box::new(UnifiedConnectorServiceGateway)
            }
            GatewayExecutionPath::ShadowUnifiedConnectorService => {
                // TODO: Implement ShadowGateway for parallel execution
                // For now, return UCS gateway
                Box::new(UnifiedConnectorServiceGateway)
            }
        }
    }
}

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
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    PaymentData: Clone + Send + Sync + 'static,
{
    // Extract execution path from context
    let execution_path = context.execution_path;

    // Create appropriate gateway based on execution path
    let gateway: Box<dyn PaymentGateway<State, ConnectorData, F, Req, Resp, PaymentData>> =
        GatewayFactory::create(execution_path);

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

#[cfg(test)]
mod tests {
    use super::*;

    // Simple mock types for testing - no complex trait implementations needed
    #[derive(Debug, Clone)]
    struct MockFlow;

    #[derive(Debug, Clone)]
    struct MockPaymentData;

    #[test]
    fn test_gateway_execution_path_equality() {
        assert_eq!(
            GatewayExecutionPath::Direct,
            GatewayExecutionPath::Direct
        );
        assert_eq!(
            GatewayExecutionPath::UnifiedConnectorService,
            GatewayExecutionPath::UnifiedConnectorService
        );
        assert_eq!(
            GatewayExecutionPath::ShadowUnifiedConnectorService,
            GatewayExecutionPath::ShadowUnifiedConnectorService
        );

        assert_ne!(
            GatewayExecutionPath::Direct,
            GatewayExecutionPath::UnifiedConnectorService
        );
        assert_ne!(
            GatewayExecutionPath::Direct,
            GatewayExecutionPath::ShadowUnifiedConnectorService
        );
    }

    #[test]
    fn test_gateway_execution_context_empty() {
        let context: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::empty();

        assert!(context.merchant_context.is_none());
        assert!(context.payment_data.is_none());
        assert!(context.header_payload.is_none());
        assert_eq!(context.execution_mode, ExecutionMode::Primary);
        assert_eq!(context.execution_path, GatewayExecutionPath::Direct);
    }

    #[test]
    fn test_gateway_execution_context_new_with_direct_path() {
        let context: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::new(
                None,
                None,
                None,
                #[cfg(feature = "v2")]
                None,
                ExecutionMode::Primary,
                GatewayExecutionPath::Direct,
            );

        assert!(context.merchant_context.is_none());
        assert!(context.payment_data.is_none());
        assert!(context.header_payload.is_none());
        assert_eq!(context.execution_mode, ExecutionMode::Primary);
        assert_eq!(context.execution_path, GatewayExecutionPath::Direct);
    }

    #[test]
    fn test_gateway_execution_context_new_with_ucs_path() {
        let context: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::new(
                None,
                None,
                None,
                #[cfg(feature = "v2")]
                None,
                ExecutionMode::Shadow,
                GatewayExecutionPath::UnifiedConnectorService,
            );

        assert!(context.merchant_context.is_none());
        assert!(context.payment_data.is_none());
        assert!(context.header_payload.is_none());
        assert_eq!(context.execution_mode, ExecutionMode::Shadow);
        assert_eq!(
            context.execution_path,
            GatewayExecutionPath::UnifiedConnectorService
        );
    }

    #[test]
    fn test_gateway_execution_context_new_with_shadow_path() {
        let context: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::new(
                None,
                None,
                None,
                #[cfg(feature = "v2")]
                None,
                ExecutionMode::Shadow,
                GatewayExecutionPath::ShadowUnifiedConnectorService,
            );

        assert_eq!(context.execution_mode, ExecutionMode::Shadow);
        assert_eq!(
            context.execution_path,
            GatewayExecutionPath::ShadowUnifiedConnectorService
        );
    }

    #[test]
    fn test_gateway_execution_path_debug_formatting() {
        let direct = GatewayExecutionPath::Direct;
        let ucs = GatewayExecutionPath::UnifiedConnectorService;
        let shadow = GatewayExecutionPath::ShadowUnifiedConnectorService;

        assert_eq!(format!("{:?}", direct), "Direct");
        assert_eq!(format!("{:?}", ucs), "UnifiedConnectorService");
        assert_eq!(format!("{:?}", shadow), "ShadowUnifiedConnectorService");
    }

    #[test]
    fn test_gateway_execution_context_clone() {
        let context1: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::new(
                None,
                None,
                None,
                #[cfg(feature = "v2")]
                None,
                ExecutionMode::Primary,
                GatewayExecutionPath::Direct,
            );

        let context2 = context1.clone();

        assert_eq!(context1.execution_mode, context2.execution_mode);
        assert_eq!(context1.execution_path, context2.execution_path);
    }

    #[test]
    fn test_direct_gateway_type_name() {
        let gateway = DirectGateway;
        let type_name = std::any::type_name_of_val(&gateway);
        assert!(type_name.contains("DirectGateway"));
    }

    #[test]
    fn test_ucs_gateway_type_name() {
        let gateway = UnifiedConnectorServiceGateway;
        let type_name = std::any::type_name_of_val(&gateway);
        assert!(type_name.contains("UnifiedConnectorServiceGateway"));
    }

    #[test]
    fn test_gateway_factory_type_name() {
        let factory = GatewayFactory;
        let type_name = std::any::type_name_of_val(&factory);
        assert!(type_name.contains("GatewayFactory"));
    }

    #[test]
    fn test_execution_path_copy_trait() {
        let path1 = GatewayExecutionPath::Direct;
        let path2 = path1; // Copy
        let path3 = path1; // Another copy

        assert_eq!(path1, path2);
        assert_eq!(path2, path3);
        assert_eq!(path1, path3);
    }

    #[test]
    fn test_context_with_different_execution_modes() {
        let primary_context: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::new(
                None,
                None,
                None,
                #[cfg(feature = "v2")]
                None,
                ExecutionMode::Primary,
                GatewayExecutionPath::Direct,
            );

        let shadow_context: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::new(
                None,
                None,
                None,
                #[cfg(feature = "v2")]
                None,
                ExecutionMode::Shadow,
                GatewayExecutionPath::UnifiedConnectorService,
            );

        assert_eq!(primary_context.execution_mode, ExecutionMode::Primary);
        assert_eq!(shadow_context.execution_mode, ExecutionMode::Shadow);
        assert_ne!(primary_context.execution_mode, shadow_context.execution_mode);
    }

    #[test]
    fn test_context_with_all_execution_paths() {
        let direct_context: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::new(
                None,
                None,
                None,
                #[cfg(feature = "v2")]
                None,
                ExecutionMode::Primary,
                GatewayExecutionPath::Direct,
            );

        let ucs_context: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::new(
                None,
                None,
                None,
                #[cfg(feature = "v2")]
                None,
                ExecutionMode::Primary,
                GatewayExecutionPath::UnifiedConnectorService,
            );

        let shadow_context: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::new(
                None,
                None,
                None,
                #[cfg(feature = "v2")]
                None,
                ExecutionMode::Shadow,
                GatewayExecutionPath::ShadowUnifiedConnectorService,
            );

        assert_eq!(direct_context.execution_path, GatewayExecutionPath::Direct);
        assert_eq!(
            ucs_context.execution_path,
            GatewayExecutionPath::UnifiedConnectorService
        );
        assert_eq!(
            shadow_context.execution_path,
            GatewayExecutionPath::ShadowUnifiedConnectorService
        );
    }

    #[test]
    fn test_execution_mode_primary_vs_shadow() {
        let primary: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::new(
                None,
                None,
                None,
                #[cfg(feature = "v2")]
                None,
                ExecutionMode::Primary,
                GatewayExecutionPath::Direct,
            );

        let shadow: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::new(
                None,
                None,
                None,
                #[cfg(feature = "v2")]
                None,
                ExecutionMode::Shadow,
                GatewayExecutionPath::Direct,
            );

        assert_eq!(primary.execution_mode, ExecutionMode::Primary);
        assert_eq!(shadow.execution_mode, ExecutionMode::Shadow);
        assert_ne!(primary.execution_mode, shadow.execution_mode);
    }

    #[test]
    fn test_empty_context_defaults() {
        let context: GatewayExecutionContext<MockFlow, MockPaymentData> =
            GatewayExecutionContext::empty();

        // Verify all fields are None/default
        assert!(context.merchant_context.is_none());
        assert!(context.payment_data.is_none());
        assert!(context.header_payload.is_none());
        
        // Verify default execution mode and path
        assert_eq!(context.execution_mode, ExecutionMode::Primary);
        assert_eq!(context.execution_path, GatewayExecutionPath::Direct);
    }

    #[test]
    fn test_gateway_structs_are_copy() {
        let direct1 = DirectGateway;
        let direct2 = direct1; // Copy
        let _direct3 = direct1; // Another copy - should compile

        let ucs1 = UnifiedConnectorServiceGateway;
        let ucs2 = ucs1; // Copy
        let _ucs3 = ucs1; // Another copy - should compile

        let factory1 = GatewayFactory;
        let factory2 = factory1; // Copy
        let _factory3 = factory1; // Another copy - should compile

        // Verify we can still use the original values
        let _ = format!("{:?}", direct2);
        let _ = format!("{:?}", ucs2);
        let _ = format!("{:?}", factory2);
    }

    #[test]
    fn test_gateway_structs_are_clone() {
        let direct = DirectGateway;
        let _direct_clone = direct.clone();

        let ucs = UnifiedConnectorServiceGateway;
        let _ucs_clone = ucs.clone();

        let factory = GatewayFactory;
        let _factory_clone = factory.clone();
    }

    #[test]
    fn test_execution_path_all_variants() {
        // Test that all variants can be created and compared
        let paths = vec![
            GatewayExecutionPath::Direct,
            GatewayExecutionPath::UnifiedConnectorService,
            GatewayExecutionPath::ShadowUnifiedConnectorService,
        ];

        assert_eq!(paths.len(), 3);
        assert!(paths.contains(&GatewayExecutionPath::Direct));
        assert!(paths.contains(&GatewayExecutionPath::UnifiedConnectorService));
        assert!(paths.contains(&GatewayExecutionPath::ShadowUnifiedConnectorService));
    }
}