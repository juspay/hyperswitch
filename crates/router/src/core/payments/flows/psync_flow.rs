use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, transformers, PaymentData},
    },
    routes::AppState,
    services,
    types::{
        self, api, storage, PaymentsRequestSyncData, PaymentsResponseData, PaymentsRouterSyncData,
    },
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::PSync,
        types::PaymentsRequestSyncData,
        types::PaymentsResponseData,
    > for PaymentData<api::PSync>
{
    async fn construct_r_d<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<
        types::RouterData<api::PSync, types::PaymentsRequestSyncData, types::PaymentsResponseData>,
    > {
        let output = transformers::construct_payment_router_data::<
            api::PSync,
            types::PaymentsRequestSyncData,
        >(state, self.clone(), connector_id, merchant_account)
        .await?;
        Ok(output.1)
    }
}

#[async_trait]
impl Feature<api::PSync, types::PaymentsRequestSyncData>
    for types::RouterData<api::PSync, types::PaymentsRequestSyncData, types::PaymentsResponseData>
{
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: api::ConnectorData,
        customer: &Option<api::CustomerResponse>,
        payment_data: PaymentData<api::PSync>,
        call_connector_action: payments::CallConnectorAction,
    ) -> (RouterResult<Self>, PaymentData<api::PSync>)
    where
        dyn api::Connector: services::ConnectorIntegration<
            api::PSync,
            types::PaymentsRequestSyncData,
            types::PaymentsResponseData,
        >,
    {
        let resp = self
            .decide_flow(
                state,
                connector,
                customer,
                Some(true),
                call_connector_action,
            )
            .await;

        (resp, payment_data)
    }
}

impl PaymentsRouterSyncData {
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a AppState,
        connector: api::ConnectorData,
        _maybe_customer: &Option<api::CustomerResponse>,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<PaymentsRouterSyncData>
    where
        dyn api::Connector + Sync: services::ConnectorIntegration<
            api::PSync,
            PaymentsRequestSyncData,
            PaymentsResponseData,
        >,
    {
        let connector_integration: services::BoxedConnectorIntegration<
            api::PSync,
            PaymentsRequestSyncData,
            PaymentsResponseData,
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
