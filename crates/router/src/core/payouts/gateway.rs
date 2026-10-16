pub mod cancel_gateway;
pub mod context;
pub mod create_gateway;
pub mod eligibility_gateway;
pub mod fulfill_gateway;
pub mod quote_gateway;
pub mod recipient_account_gateway;
pub mod recipient_gateway;
pub mod sync_gateway;

use hyperswitch_interfaces::{
    api::gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};

use crate::{core::unified_connector_service::kill_switch, services, types, SessionState};

/// Execute a payout gateway and record only shadow transport/proxy failures.
pub async fn execute_payout_gateway<F, ConnectorData, Req, Resp, Context, FlowOutput>(
    state: &SessionState,
    connector_integration: BoxedConnectorIntegrationInterface<F, ConnectorData, Req, Resp>,
    router_data: &types::RouterData<F, Req, Resp>,
    call_connector_action: common_enums::CallConnectorAction,
    connector_request: Option<services::Request>,
    return_raw_connector_response: Option<bool>,
    context: Context,
) -> common_utils::errors::CustomResult<FlowOutput, ConnectorError>
where
    ConnectorData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
    F: Clone
        + std::fmt::Debug
        + Send
        + Sync
        + 'static
        + gateway::PayoutFlowGateway<SessionState, ConnectorData, Req, Resp, Context, FlowOutput>,
    Req: std::fmt::Debug + Clone + Send + Sync + serde::Serialize + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + serde::Serialize + 'static,
    Context: gateway::GatewayContext + 'static,
    FlowOutput: Clone + Send + Sync + gateway::GetRouterData<F, Req, Resp> + 'static,
{
    let execution_path = context.execution_path();
    let result = gateway::execute_payout_gateway(
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
        kill_switch::shadow::record_failure::<F, Req, Resp, FlowOutput>(
            state,
            &router_data.payment_id,
            &result,
        )
        .await;
    }
    result
}
