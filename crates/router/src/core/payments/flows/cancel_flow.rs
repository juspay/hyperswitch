use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, transformers, PaymentData},
    },
    routes::AppState,
    services,
    types::{self, api, storage, PaymentsCancelRouterData, PaymentsResponseData},
};

#[async_trait]
impl ConstructFlowSpecificData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for PaymentData<api::Void>
{
    async fn construct_r_d<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<PaymentsCancelRouterData> {
        let output = transformers::construct_payment_router_data::<
            api::Void,
            types::PaymentsCancelData,
        >(state, self.clone(), connector_id, merchant_account)
        .await?;
        Ok(output.1)
    }
}

#[async_trait]
impl Feature<api::Void, types::PaymentsCancelData>
    for types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
{
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: api::ConnectorData,
        customer: &Option<api::CustomerResponse>,
        payment_data: PaymentData<api::Void>,
        call_connector_action: payments::CallConnectorAction,
    ) -> (RouterResult<Self>, PaymentData<api::Void>)
    where
        dyn api::Connector: services::ConnectorIntegration<
            api::Void,
            types::PaymentsCancelData,
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

impl PaymentsCancelRouterData {
    #[allow(clippy::too_many_arguments)]
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &AppState,
        connector: api::ConnectorData,
        _maybe_customer: &Option<api::CustomerResponse>,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<PaymentsCancelRouterData>
    where
        // P: 'a,
        dyn api::Connector + Sync: services::ConnectorIntegration<
            api::Void,
            types::PaymentsCancelData,
            PaymentsResponseData,
        >,
    {
        let connector_integration: services::BoxedConnectorIntegration<
            api::Void,
            types::PaymentsCancelData,
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
