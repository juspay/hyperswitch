pub mod access_token_gateway;
pub mod authenticate_gateway;
pub mod authorize_gateway;
pub mod cancel_gateway;
pub mod capture_gateway;
pub mod complete_authorize_gateway;
pub mod context;
pub mod create_customer_gateway;
pub mod create_order_gateway;
pub mod generate_qr_gateway;
pub mod incremental_authorization_gateway;
pub mod payment_method_token_create_gateway;
pub mod post_authenticate_gateway;
pub mod post_capture_void_gateway;
pub mod post_capture_void_sync_gateway;
pub mod pre_authenticate_gateway;
pub mod pre_authorize_void_gateway;
pub mod psync_gateway;
pub mod push_notification_gateway;
pub mod session_gateway;
pub mod session_token_gateway;
pub mod setup_mandate;
use std::sync;

use common_enums;
use hyperswitch_domain_models::{router_data_v2::PaymentFlowData, router_flow_types::payments};
use hyperswitch_interfaces::{
    api::{gateway, Connector, ConnectorIntegration},
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    connector_integration_v2::{ConnectorIntegrationV2, ConnectorV2},
    errors::ConnectorError,
};

use crate::{
    core::{
        errors::utils::ConnectorErrorExt, payments::gateway::context as gateway_context,
        unified_connector_service::kill_switch,
    },
    errors::RouterResult,
    services, types,
    types::api,
    SessionState,
};

pub static GRANULAR_GATEWAY_SUPPORTED_FLOWS: sync::LazyLock<Vec<&'static str>> =
    sync::LazyLock::new(|| {
        vec![
            std::any::type_name::<payments::PSync>(),
            std::any::type_name::<payments::Authorize>(),
            std::any::type_name::<payments::CompleteAuthorize>(),
            std::any::type_name::<payments::SetupMandate>(),
        ]
    });

/// Execute a payment gateway and record qualifying request failures on the shadow path.
pub fn execute_payment_gateway<'a, F, ConnectorData, Req, Resp, Context, FlowOutput>(
    state: &'a SessionState,
    connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
    router_data: &'a types::RouterData<F, Req, Resp>,
    call_connector_action: common_enums::CallConnectorAction,
    connector_request: Option<services::Request>,
    return_raw_connector_response: Option<bool>,
    context: Context,
) -> impl std::future::Future<Output = common_utils::errors::CustomResult<FlowOutput, ConnectorError>>
       + Send
       + 'a
where
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone
        + std::fmt::Debug
        + Send
        + Sync
        + 'static
        + gateway::FlowGateway<SessionState, ConnectorData, Req, Resp, Context, FlowOutput>,
    Req: std::fmt::Debug + Clone + Send + Sync + serde::Serialize + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + serde::Serialize + 'static,
    Context: gateway::GatewayContext + 'static,
    FlowOutput: Clone + Send + Sync + gateway::GetRouterData<F, Req, Resp> + 'static,
{
    // Keep the gateway result and post-execution counter check out of callers' stack frames.
    Box::pin(async move {
        let execution_path = context.execution_path();
        let result = gateway::execute_payment_gateway(
            state,
            connector_integration,
            router_data,
            call_connector_action,
            connector_request,
            return_raw_connector_response,
            context,
        )
        .await;
        if matches!(
            execution_path,
            common_enums::ExecutionPath::ShadowUnifiedConnectorService
        ) {
            kill_switch::shadow::check_record_failure::<F, Req, Resp, FlowOutput>(
                state,
                &router_data.payment_id,
                &result,
            )
            .await;
        }
        result
    })
}

pub async fn handle_gateway_call<Flow, Req, Resp, ResourceCommonData, FlowOutput>(
    state: &SessionState,
    router_data: types::RouterData<Flow, Req, Resp>,
    connector: &api::ConnectorData,
    gateway_context: &gateway_context::RouterGatewayContext,
    call_connector_action: common_enums::CallConnectorAction,
    connector_request: Option<services::Request>,
    return_raw_connector_response: Option<bool>,
) -> RouterResult<FlowOutput>
where
    Flow: gateway::FlowGateway<
        SessionState,
        PaymentFlowData,
        Req,
        Resp,
        gateway_context::RouterGatewayContext,
        FlowOutput,
    >,
    FlowOutput: Clone + Send + Sync + gateway::GetRouterData<Flow, Req, Resp> + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + serde::Serialize + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + serde::Serialize + 'static,
    dyn Connector + Sync: ConnectorIntegration<Flow, Req, Resp>,
    dyn ConnectorV2 + Sync: ConnectorIntegrationV2<Flow, PaymentFlowData, Req, Resp>,
{
    let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
        Flow,
        Req,
        Resp,
    > = connector.connector.get_connector_integration();
    // TODO: Handle gateway_context later
    let resp = execute_payment_gateway(
        state,
        connector_integration,
        &router_data,
        call_connector_action,
        connector_request,
        return_raw_connector_response,
        gateway_context.clone(),
    )
    .await
    .to_payment_failed_response()?;
    Ok(resp)
}
