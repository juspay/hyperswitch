pub mod authorize_flow;
pub mod cancel_flow;
pub mod capture_flow;
pub mod psync_flow;
pub mod session_flow;
pub mod verfiy_flow;

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
pub trait ConstructFlowSpecificData<F, Req, Res> {
    async fn construct_router_data<'a>(
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
        connector: &api::ConnectorData,
        maybe_customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<Self>
    where
        Self: Sized,
        F: Clone,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;
}
