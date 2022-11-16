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
        self, api, storage, PaymentsRequestCaptureData, PaymentsResponseData,
        PaymentsRouterCaptureData,
    },
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::PCapture,
        types::PaymentsRequestCaptureData,
        types::PaymentsResponseData,
    > for PaymentData<api::PCapture>
{
    async fn construct_r_d<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<PaymentsRouterCaptureData> {
        let output = transformers::construct_payment_router_data::<
            api::PCapture,
            types::PaymentsRequestCaptureData,
        >(state, self.clone(), connector_id, merchant_account)
        .await?;
        Ok(output.1)
    }
}

#[async_trait]
impl Feature<api::PCapture, types::PaymentsRequestCaptureData>
    for types::RouterData<
        api::PCapture,
        types::PaymentsRequestCaptureData,
        types::PaymentsResponseData,
    >
{
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: api::ConnectorData,
        customer: &Option<api::CustomerResponse>,
        payment_data: PaymentData<api::PCapture>,
        call_connector_action: payments::CallConnectorAction,
    ) -> (RouterResult<Self>, PaymentData<api::PCapture>)
    where
        dyn api::Connector: services::ConnectorIntegration<
            api::PCapture,
            types::PaymentsRequestCaptureData,
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

impl PaymentsRouterCaptureData {
    #[allow(clippy::too_many_arguments)]
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a AppState,
        connector: api::ConnectorData,
        _maybe_customer: &Option<api::CustomerResponse>,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<PaymentsRouterCaptureData>
    where
        dyn api::Connector + Sync: services::ConnectorIntegration<
            api::PCapture,
            PaymentsRequestCaptureData,
            PaymentsResponseData,
        >,
    {
        let connector_integration: services::BoxedConnectorIntegration<
            api::PCapture,
            PaymentsRequestCaptureData,
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
