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
use common_utils::{
    errors::{CustomResult, ErrorSwitch},
    request::Request,
};
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse, router_data::RouterData,
};
use hyperswitch_masking::Secret;
use router_env::{logger, tracing::Instrument};

use crate::{
    api_client::{self, ApiClientWrapper},
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
    helpers,
    unified_connector_service::transformers::UnifiedConnectorServiceError,
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

/// Trait to extract RouterData from various output types
///
pub trait GetRouterData<F, Req, Resp> {
    /// Get a reference to the RouterData
    fn get_router_data(&self) -> &RouterData<F, Req, Resp>;
}

impl<F, Req, Resp> GetRouterData<F, Req, Resp> for RouterData<F, Req, Resp> {
    fn get_router_data(&self) -> &Self {
        self
    }
}

impl<F, Req, Resp, T> GetRouterData<F, Req, Resp> for (RouterData<F, Req, Resp>, Option<T>) {
    fn get_router_data(&self) -> &RouterData<F, Req, Resp> {
        &self.0
    }
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
pub trait PaymentGateway<
    State,
    ConnectorData,
    F,
    Req,
    Resp,
    Context,
    FlowOutput = RouterData<F, Req, Resp>,
>: Send + Sync where
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
    ) -> CustomResult<FlowOutput, ConnectorError>;
}

/// Payout gateway trait
///
/// Defines the interface for executing payout operations through different gateway types.
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
pub trait PayoutGateway<
    State,
    ConnectorData,
    F,
    Req,
    Resp,
    Context,
    FlowOutput = RouterData<F, Req, Resp>,
>: Send + Sync where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    Context: GatewayContext,
{
    /// Execute payout gateway operation
    async fn execute(
        self: Box<Self>,
        state: &State,
        connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
        router_data: &RouterData<F, Req, Resp>,
        call_connector_action: CallConnectorAction,
        connector_request: Option<Request>,
        return_raw_connector_response: Option<bool>,
        context: Context,
    ) -> CustomResult<FlowOutput, ConnectorError>;
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

#[async_trait]
impl<State, ConnectorData, F, Req, Resp, Context>
    PayoutGateway<State, ConnectorData, F, Req, Resp, Context> for DirectGateway
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

#[async_trait]
impl<State, ConnectorData, F, Req, Resp, Context, T>
    PaymentGateway<
        State,
        ConnectorData,
        F,
        Req,
        Resp,
        Context,
        (RouterData<F, Req, Resp>, Option<T>),
    > for DirectGateway
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
    ) -> CustomResult<(RouterData<F, Req, Resp>, Option<T>), ConnectorError> {
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
        .map(|router_data| (router_data, None))
    }
}

/// Flow gateway trait for determining execution path
///
/// This trait allows flows to specify which gateway implementation should be used
/// based on the execution path. Each flow implements this trait to provide
/// flow-specific gateway selection logic.
pub trait FlowGateway<
    State,
    ConnectorData,
    Req,
    Resp,
    Context,
    FlowOutput = RouterData<Self, Req, Resp>,
>: Clone + std::fmt::Debug + Send + Sync + 'static where
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
    ) -> Box<dyn PaymentGateway<State, ConnectorData, Self, Req, Resp, Context, FlowOutput>>;
}

/// Payout Flow gateway trait for determining execution path
///
/// This trait allows flows to specify which gateway implementation should be used
/// based on the execution path. Each flow implements this trait to provide
/// flow-specific gateway selection logic.
pub trait PayoutFlowGateway<
    State,
    ConnectorData,
    Req,
    Resp,
    Context,
    FlowOutput = RouterData<Self, Req, Resp>,
>: Clone + std::fmt::Debug + Send + Sync + 'static where
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
    fn get_payout_gateway(
        execution_path: ExecutionPath,
    ) -> Box<dyn PayoutGateway<State, ConnectorData, Self, Req, Resp, Context, FlowOutput>>;
}

/// Execute payment gateway operation (backward compatible version)
///
/// This version maintains backward compatibility by using direct execution when no context is provided.
/// Use `execute_payment_gateway_with_context` for UCS support.
pub async fn execute_payment_gateway<State, ConnectorData, F, Req, Resp, Context, FlowOutput>(
    state: &State,
    connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
    router_data: &RouterData<F, Req, Resp>,
    call_connector_action: CallConnectorAction,
    connector_request: Option<Request>,
    return_raw_connector_response: Option<bool>,
    context: Context,
) -> CustomResult<FlowOutput, ConnectorError>
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper + helpers::GetComparisonServiceConfig,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone
        + std::fmt::Debug
        + Send
        + Sync
        + 'static
        + FlowGateway<State, ConnectorData, Req, Resp, Context, FlowOutput>,
    Req: std::fmt::Debug + Clone + Send + Sync + serde::Serialize + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + serde::Serialize + 'static,
    Context: GatewayContext + 'static,
    FlowOutput: Clone + Send + Sync + GetRouterData<F, Req, Resp> + 'static,
{
    let execution_path = context.execution_path();

    match execution_path {
        ExecutionPath::Direct => {
            let gateway: Box<
                dyn PaymentGateway<State, ConnectorData, F, Req, Resp, Context, FlowOutput>,
            > = F::get_gateway(ExecutionPath::Direct);
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
        }
        ExecutionPath::UnifiedConnectorService => {
            let gateway: Box<
                dyn PaymentGateway<State, ConnectorData, F, Req, Resp, Context, FlowOutput>,
            > = F::get_gateway(ExecutionPath::UnifiedConnectorService);

            // Execute through selected gateway
            let ucs_result = gateway
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
                .attach_printable("Gateway execution failed")?;
            Ok(ucs_result)
        }
        ExecutionPath::ShadowUnifiedConnectorService => {
            let gateway: Box<
                dyn PaymentGateway<State, ConnectorData, F, Req, Resp, Context, FlowOutput>,
            > = F::get_gateway(ExecutionPath::Direct);
            let direct_result = gateway
                .execute(
                    state,
                    connector_integration.clone_box(),
                    router_data,
                    call_connector_action.clone(),
                    connector_request,
                    return_raw_connector_response,
                    context.clone(),
                )
                .await;

            let direct_for_compare: Result<RouterData<F, Req, Resp>, String> = match &direct_result
            {
                Ok(fo) => Ok(fo.get_router_data().clone()),
                Err(e) => Err(format!("{:?}", e)),
            };
            let state_clone = state.clone();
            let router_data_clone = router_data.clone();
            let return_raw_connector_response_clone = return_raw_connector_response;
            let context_clone = context;
            let connector_name = router_data.connector.clone();
            let merchant_id = router_data.merchant_id.clone();
            let payment_method = router_data.payment_method;
            let payment_method_type = router_data.payment_method_type;
            tokio::spawn(
                async move {
                    let gateway: Box<
                        dyn PaymentGateway<State, ConnectorData, F, Req, Resp, Context, FlowOutput>,
                    > = F::get_gateway(ExecutionPath::ShadowUnifiedConnectorService);
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

                    if let Err(e) = &ucs_shadow_result {
                        logger::warn!(
                            error=?e,
                            execution_mode="shadow",
                            primary_impact=false,
                            "UCS shadow execution failed; primary execution result is unaffected"
                        );
                    }

                    let ucs_for_compare = match ucs_shadow_result {
                        Ok(fo) => Ok(fo.get_router_data().clone()),
                        Err(e) => Err(format!("{:?}", e)),
                    };

                    helpers::serialize_comparison_results_and_send(
                        &state_clone,
                        connector_name,
                        direct_for_compare,
                        ucs_for_compare,
                        Some(&merchant_id),
                        Some(payment_method),
                        payment_method_type,
                    )
                    .await;
                }
                .instrument(router_env::tracing::Span::current()),
            );
            direct_result
        }
    }
}

/// Execute payout gateway operation (backward compatible version)
///
/// This version maintains backward compatibility by using direct execution when no context is provided.
/// Use `execute_payout_gateway_with_context` for UCS support.
pub async fn execute_payout_gateway<State, ConnectorData, F, Req, Resp, Context, FlowOutput>(
    state: &State,
    connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
    router_data: &RouterData<F, Req, Resp>,
    call_connector_action: CallConnectorAction,
    connector_request: Option<Request>,
    return_raw_connector_response: Option<bool>,
    context: Context,
) -> CustomResult<FlowOutput, ConnectorError>
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper + helpers::GetComparisonServiceConfig,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone
        + std::fmt::Debug
        + Send
        + Sync
        + 'static
        + PayoutFlowGateway<State, ConnectorData, Req, Resp, Context, FlowOutput>,
    Req: std::fmt::Debug + Clone + Send + Sync + serde::Serialize + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + serde::Serialize + 'static,
    Context: GatewayContext + 'static,
    FlowOutput: Clone + Send + Sync + GetRouterData<F, Req, Resp> + 'static,
{
    let execution_path = context.execution_path();

    match execution_path {
        ExecutionPath::Direct => {
            let gateway: Box<
                dyn PayoutGateway<State, ConnectorData, F, Req, Resp, Context, FlowOutput>,
            > = F::get_payout_gateway(ExecutionPath::Direct);
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
        }
        ExecutionPath::UnifiedConnectorService => {
            let gateway: Box<
                dyn PayoutGateway<State, ConnectorData, F, Req, Resp, Context, FlowOutput>,
            > = F::get_payout_gateway(ExecutionPath::UnifiedConnectorService);

            // Execute through selected gateway
            let ucs_result = gateway
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
                .attach_printable("Gateway execution failed")?;
            Ok(ucs_result)
        }
        ExecutionPath::ShadowUnifiedConnectorService => {
            let gateway: Box<
                dyn PayoutGateway<State, ConnectorData, F, Req, Resp, Context, FlowOutput>,
            > = F::get_payout_gateway(ExecutionPath::Direct);
            let direct_result = gateway
                .execute(
                    state,
                    connector_integration.clone_box(),
                    router_data,
                    call_connector_action.clone(),
                    connector_request,
                    return_raw_connector_response,
                    context.clone(),
                )
                .await;

            let direct_for_compare: Result<RouterData<F, Req, Resp>, String> = match &direct_result
            {
                Ok(fo) => Ok(fo.get_router_data().clone()),
                Err(e) => Err(format!("{:?}", e)),
            };
            let state_clone = state.clone();
            let router_data_clone = router_data.clone();
            let return_raw_connector_response_clone = return_raw_connector_response;
            let context_clone = context;
            let connector_name = router_data.connector.clone();
            let merchant_id = router_data.merchant_id.clone();
            let payment_method = router_data.payment_method;
            let payment_method_type = router_data.payment_method_type;
            tokio::spawn(
                async move {
                    let gateway: Box<
                        dyn PayoutGateway<State, ConnectorData, F, Req, Resp, Context, FlowOutput>,
                    > = F::get_payout_gateway(ExecutionPath::ShadowUnifiedConnectorService);
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

                    if let Err(e) = &ucs_shadow_result {
                        logger::warn!(
                            error=?e,
                            execution_mode="shadow",
                            primary_impact=false,
                            "UCS shadow execution failed; primary execution result is unaffected"
                        );
                    }

                    let ucs_for_compare = match ucs_shadow_result {
                        Ok(fo) => Ok(fo.get_router_data().clone()),
                        Err(e) => Err(format!("{:?}", e)),
                    };

                    helpers::serialize_comparison_results_and_send(
                        &state_clone,
                        connector_name,
                        direct_for_compare,
                        ucs_for_compare,
                        Some(&merchant_id),
                        Some(payment_method),
                        payment_method_type,
                    )
                    .await;
                }
                .instrument(router_env::tracing::Span::current()),
            );
            direct_result
        }
    }
}

/// A dispute action executed through the direct connector or UCS.
#[async_trait]
pub trait DisputeGateway<State, ConnectorData, F, Req, Resp, Context>: Send + Sync
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    Context: GatewayContext,
{
    /// Execute one dispute action through this gateway implementation.
    async fn execute(
        self: Box<Self>,
        state: &State,
        connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
        router_data: &RouterData<F, Req, Resp>,
        context: Context,
    ) -> CustomResult<RouterData<F, Req, Resp>, ApiErrorResponse>;
}

/// Selects the gateway implementation for a dispute flow.
pub trait DisputeFlowGateway<State, ConnectorData, Req, Resp, Context>:
    Clone + std::fmt::Debug + Send + Sync + 'static
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper,
    ConnectorData: Clone + RouterDataConversion<Self, Req, Resp> + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    Context: GatewayContext,
{
    /// Select the direct or UCS gateway for this dispute flow.
    fn get_dispute_gateway(
        execution_path: ExecutionPath,
    ) -> Box<dyn DisputeGateway<State, ConnectorData, Self, Req, Resp, Context>>;
}

fn dispute_comparison_value<F, Req, Resp>(
    result: &CustomResult<RouterData<F, Req, Resp>, ApiErrorResponse>,
) -> serde_json::Value
where
    Resp: std::fmt::Debug,
{
    match result {
        Ok(data) => serde_json::json!({
            "connector_http_status_code": data.connector_http_status_code,
            "response": format!("{:?}", data.response),
        }),
        Err(error) => serde_json::json!({ "error": format!("{error:?}") }),
    }
}

/// Run a dispute through the selected gateway. The shadow path returns the direct result
/// and compares only outcomes, keeping dispute evidence out of the comparison payload.
pub async fn execute_dispute_gateway<State, ConnectorData, F, Req, Resp, Context>(
    state: &State,
    ucs_state: &State,
    connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
    router_data: &RouterData<F, Req, Resp>,
    context: Context,
) -> CustomResult<RouterData<F, Req, Resp>, ApiErrorResponse>
where
    State: Clone + Send + Sync + 'static + ApiClientWrapper + helpers::GetComparisonServiceConfig,
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: DisputeFlowGateway<State, ConnectorData, Req, Resp, Context>,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    Context: GatewayContext + 'static,
{
    match context.execution_path() {
        ExecutionPath::Direct => {
            F::get_dispute_gateway(ExecutionPath::Direct)
                .execute(state, connector_integration, router_data, context)
                .await
        }
        ExecutionPath::UnifiedConnectorService => {
            F::get_dispute_gateway(ExecutionPath::UnifiedConnectorService)
                .execute(ucs_state, connector_integration, router_data, context)
                .await
        }
        ExecutionPath::ShadowUnifiedConnectorService => {
            let direct_result = F::get_dispute_gateway(ExecutionPath::Direct)
                .execute(
                    state,
                    connector_integration.clone_box(),
                    router_data,
                    context.clone(),
                )
                .await;
            let direct_for_compare = dispute_comparison_value(&direct_result);
            let shadow_state = ucs_state.clone();
            let shadow_router_data = router_data.clone();
            let connector_name = router_data.connector.clone();
            let merchant_id = router_data.merchant_id.clone();
            let payment_method = router_data.payment_method;
            let payment_method_type = router_data.payment_method_type;
            tokio::spawn(
                async move {
                    let shadow_result =
                        F::get_dispute_gateway(ExecutionPath::ShadowUnifiedConnectorService)
                            .execute(
                                &shadow_state,
                                connector_integration,
                                &shadow_router_data,
                                context,
                            )
                            .await;
                    if let Err(error) = &shadow_result {
                        logger::warn!(
                            ?error,
                            execution_mode = "shadow",
                            primary_impact = false,
                            "UCS dispute shadow execution failed"
                        );
                    }
                    if let Some(config) = shadow_state.get_comparison_service_config() {
                        let comparison_data = helpers::ComparisonData {
                            hyperswitch_data: Secret::new(direct_for_compare),
                            unified_connector_service_data: Secret::new(dispute_comparison_value(
                                &shadow_result,
                            )),
                        };
                        if let Err(error) = helpers::send_comparison_data(
                            &shadow_state,
                            comparison_data,
                            config,
                            connector_name,
                            api_client::get_flow_name::<F>().ok(),
                            shadow_state.get_request_id_str(),
                            Some(&merchant_id),
                            Some(payment_method),
                            payment_method_type,
                        )
                        .await
                        {
                            logger::warn!(?error, "Failed to send dispute shadow comparison");
                        }
                    }
                }
                .instrument(router_env::tracing::Span::current()),
            );
            direct_result
        }
    }
}

/// Converts a `Report<UnifiedConnectorServiceError>` into a `Report<ConnectorError>`
/// using the `ErrorSwitch` trait for consistent, exhaustive mapping.
///
/// Shared by all payment gateway files (authorize, psync, capture, void) to avoid duplication.
pub fn convert_ucs_error_to_connector_error(
    report: Report<UnifiedConnectorServiceError>,
) -> Report<ConnectorError> {
    let connector_error = report.current_context().switch();
    report.change_context(connector_error)
}
