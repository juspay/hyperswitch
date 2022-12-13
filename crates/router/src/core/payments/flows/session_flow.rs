use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, transformers, PaymentData},
    },
    routes, services,
    types::{
        self, api,
        storage::{self, enums},
    },
};

#[async_trait]
impl
    ConstructFlowSpecificData<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for PaymentData<api::Session>
{
    async fn construct_router_data<'a>(
        &self,
        state: &routes::AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::PaymentsSessionRouterData> {
        transformers::construct_payment_router_data::<api::Session, types::PaymentsSessionData>(
            state,
            self.clone(),
            connector_id,
            merchant_account,
        )
        .await
    }
}

#[async_trait]
impl Feature<api::Session, types::PaymentsSessionData> for types::PaymentsSessionRouterData {
    async fn decide_flows<'a>(
        self,
        state: &routes::AppState,
        connector: &api::ConnectorData,
        customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        _storage_schema: enums::MerchantStorageScheme,
    ) -> RouterResult<Self> {
        self.decide_flow(
            state,
            connector,
            customer,
            Some(true),
            call_connector_action,
        )
        .await
    }
}

impl types::PaymentsSessionRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a routes::AppState,
        connector: &api::ConnectorData,
        _customer: &Option<storage::Customer>,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<types::PaymentsSessionRouterData> {
        let connector_integration: services::BoxedConnectorIntegration<
            api::Session,
            types::PaymentsSessionData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            self,
            call_connector_action,
        )
        .await
        .map_err(|error| error.to_payment_failed_response())?;

        Ok(resp)
    }
}
