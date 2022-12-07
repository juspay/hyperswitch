mod authorize_flow;
mod cancel_flow;
mod capture_flow;
mod psync_flow;
mod verfiy_flow;

use async_trait::async_trait;

use super::PaymentData;
use crate::{
    core::{errors::RouterResult, payments},
    routes::AppState,
    services,
    types::{self, api, storage},
};

#[async_trait]
pub trait ConstructFlowSpecificData<F, Req, Res> {
    async fn construct_r_d<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::RouterData<F, Req, Res>>;
}

#[async_trait]
pub trait Feature<F, T> {
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: api::ConnectorData,
        maybe_customer: &Option<api::CustomerResponse>,
        payment_data: PaymentData<F>,
        call_connector_action: payments::CallConnectorAction,
    ) -> (RouterResult<Self>, PaymentData<F>)
    where
        Self: std::marker::Sized,
        F: Clone,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;
}
