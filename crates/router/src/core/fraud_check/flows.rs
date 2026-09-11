pub mod checkout_flow;
pub mod fulfillment_flow;
pub mod record_return;
pub mod sale_flow;
pub mod transaction_flow;

use async_trait::async_trait;

use crate::{
    core::{
        errors::RouterResult,
        payments::{self, flows::ConstructFlowSpecificData},
    },
    routes::SessionState,
    services,
    types::{
        api::{Connector, FraudCheckConnectorData},
        domain,
        fraud_check::FraudCheckResponseData,
    },
};

#[async_trait]
pub trait FeatureFrm<F, T> {
    /// Execute this flow against the connector.
    ///
    /// `gateway_context` carries the execution path decided by
    /// `should_call_unified_connector_service`. Flows with a UCS equivalent
    /// dispatch on it through `execute_payment_gateway`; the notify-style flows
    /// have none and stay on the direct path regardless.
    async fn decide_frm_flows<'a>(
        self,
        state: &SessionState,
        connector: &FraudCheckConnectorData,
        call_connector_action: payments::CallConnectorAction,
        platform: &domain::Platform,
        gateway_context: payments::gateway::context::RouterGatewayContext,
    ) -> RouterResult<Self>
    where
        Self: Sized,
        F: Clone,
        dyn Connector: services::ConnectorIntegration<F, T, FraudCheckResponseData>;
}
