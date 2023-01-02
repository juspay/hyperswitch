mod authorize_flow;
mod cancel_flow;
mod capture_flow;
mod psync_flow;
mod session_flow;
mod verfiy_flow;

use async_trait::async_trait;

use crate::{
    core::{errors::RouterResult, payments},
    routes::AppState,
    services,
    types::{
        self, api,
        storage::{self, enums},
    },
};

#[async_trait]
pub trait ConstructFlowSpecificData<'st, F, Req, Res> {
    async fn construct_router_data(
        &self,
        state: &'st AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::RouterData<'st, F, Req, Res>>;
}

#[async_trait]
pub trait Feature<'st, F, T> {
    type Output<'a>;
    async fn decide_flows(
        self,
        state: &'st AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<Self::Output<'st>>
    where
        Self: Sized,
        F: Clone,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;
}
