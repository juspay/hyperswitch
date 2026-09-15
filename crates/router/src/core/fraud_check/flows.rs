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
    async fn decide_frm_flows<'a>(
        self,
        state: &SessionState,
        connector: &FraudCheckConnectorData,
        call_connector_action: payments::CallConnectorAction,
        platform: &domain::Platform,
    ) -> RouterResult<Self>
    where
        Self: Sized,
        F: Clone,
        dyn Connector: services::ConnectorIntegration<F, T, FraudCheckResponseData>;

    /// Execute this flow on the Unified Connector Service.
    ///
    /// Only flows with a UCS equivalent override this; the default keeps the
    /// notify-style flows on the direct path. A UCS-backed provider has no
    /// in-process connector, so there is nothing to fall back to — the caller
    /// must already have decided the flow is routable to UCS.
    #[cfg(feature = "v1")]
    async fn decide_frm_flows_via_ucs<'a>(
        self,
        _state: &SessionState,
        _gateway_context: payments::gateway::context::RouterGatewayContext,
    ) -> RouterResult<Self>
    where
        Self: Sized,
    {
        Err(crate::core::errors::ApiErrorResponse::NotImplemented {
            message: crate::core::errors::NotImplementedMessage::Reason(
                "this FRM flow has no Unified Connector Service equivalent".to_string(),
            ),
        }
        .into())
    }
}
