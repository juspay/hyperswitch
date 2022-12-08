use async_trait::async_trait;

use super::ConstructFlowSpecificData;
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, transformers, Feature, PaymentData},
    },
    routes::AppState,
    services,
    types::{
        self, api, storage, PaymentsCaptureData, PaymentsCaptureRouterData, PaymentsResponseData,
    },
};

#[async_trait]
impl
    ConstructFlowSpecificData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for PaymentData<api::Capture>
{
    async fn construct_r_d<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<PaymentsCaptureRouterData> {
        let output = transformers::construct_payment_router_data::<
            api::Capture,
            types::PaymentsCaptureData,
        >(state, self.clone(), connector_id, merchant_account)
        .await?;
        Ok(output.1)
    }
}

#[async_trait]
impl Feature<api::Capture, types::PaymentsCaptureData>
    for types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
{
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: api::ConnectorData,
        customer: &Option<storage::Customer>,
        payment_data: PaymentData<api::Capture>,
        call_connector_action: payments::CallConnectorAction,
    ) -> (RouterResult<Self>, PaymentData<api::Capture>)
    where
        dyn api::Connector: services::ConnectorIntegration<
            api::Capture,
            types::PaymentsCaptureData,
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

impl PaymentsCaptureRouterData {
    #[allow(clippy::too_many_arguments)]
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a AppState,
        connector: api::ConnectorData,
        _maybe_customer: &Option<storage::Customer>,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<PaymentsCaptureRouterData>
    where
        dyn api::Connector + Sync:
            services::ConnectorIntegration<api::Capture, PaymentsCaptureData, PaymentsResponseData>,
    {
        let connector_integration: services::BoxedConnectorIntegration<
            api::Capture,
            PaymentsCaptureData,
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
