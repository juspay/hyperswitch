pub mod access_token_gateway;
pub mod authenticate_gateway;
pub mod authorize_gateway;
pub mod context;
pub mod create_customer_gateway;
pub mod create_order_gateway;
pub mod payment_method_token_create_gateway;
pub mod post_authenticate_gateway;
pub mod pre_authenticate_gateway;
pub mod psync_gateway;
pub mod session_token_gateway;
pub mod setup_mandate;
use std::sync;

use hyperswitch_domain_models::router_flow_types::payments;
use hyperswitch_interfaces::api::gateway;
use crate::core::payments::gateway::context as gateway_context;

pub static GRANULAR_GATEWAY_SUPPORTED_FLOWS: sync::LazyLock<Vec<&'static str>> =
    sync::LazyLock::new(|| {
        vec![
            std::any::type_name::<payments::PSync>(),
            std::any::type_name::<payments::Authorize>(),
            std::any::type_name::<payments::SetupMandate>(),
        ]
    });

pub async fn handle_gateway_call<Flow, Req, Resp>(
    state: &SessionState,
    router_data: types::RouterData<
        Flow,
        Req,
        Resp,
    >,
    connector: &api::ConnectorData,
    gateway_context: &gateway_context::RouterGatewayContext,
) -> RouterResult<
        Flow,
        Req,
        Resp,
    >
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
        payments::CallConnectorAction::Trigger,
        None,
        None,
        gateway_context.clone(),
    )
    .await
    .to_payment_failed_response()?;
    Ok(resp)
}