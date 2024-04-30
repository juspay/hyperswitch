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
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<Self>
    where
        Self: Sized,
        F: Clone,
        dyn Connector: services::ConnectorIntegration<F, T, FraudCheckResponseData>;
}
