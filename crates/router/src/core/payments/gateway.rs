pub mod access_token_gateway;
pub mod authenticate_gateway;
pub mod authorize_gateway;
pub mod cancel_gateway;
pub mod capture_gateway;
pub mod complete_authorize_gateway;
pub mod context;
pub mod create_customer_gateway;
pub mod create_order_gateway;
pub mod incremental_authorization_gateway;
pub mod payment_method_token_create_gateway;
pub mod post_authenticate_gateway;
pub mod pre_authenticate_gateway;
pub mod psync_gateway;
pub mod session_gateway;
pub mod session_token_gateway;
pub mod setup_mandate;
use std::sync;

use common_enums;
use hyperswitch_domain_models::{router_data_v2::PaymentFlowData, router_flow_types::payments};
use hyperswitch_interfaces::{
    api::{gateway, Connector, ConnectorIntegration},
    connector_integration_v2::{ConnectorIntegrationV2, ConnectorV2},
};

use crate::{
    core::{errors::utils::ConnectorErrorExt, payments::gateway::context as gateway_context},
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

pub async fn handle_gateway_call<Flow, Req, Resp, ResourceCommonData, FlowOutput>(
    state: &SessionState,
    router_data: types::RouterData<Flow, Req, Resp>,
    connector: &api::ConnectorData,
    gateway_context: &gateway_context::RouterGatewayContext,
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
    let resp = gateway::execute_payment_gateway(
        state,
        connector_integration,
        &router_data,
        common_enums::CallConnectorAction::Trigger,
        None,
        None,
        gateway_context.clone(),
    )
    .await
    .to_payment_failed_response()?;
    Ok(resp)
}
