use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
    },
    routes::{metrics, AppState},
    services,
    types::{self, api, domain},
};

#[async_trait]
impl ConstructFlowSpecificData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for PaymentData<api::Void>
{
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> RouterResult<types::PaymentsCancelRouterData> {
        Box::pin(transformers::construct_payment_router_data::<
            api::Void,
            types::PaymentsCancelData,
        >(
            state,
            self.clone(),
            connector_id,
            merchant_account,
            key_store,
            customer,
            merchant_connector_account,
        ))
        .await
    }
}

#[async_trait]
impl Feature<api::Void, types::PaymentsCancelData>
    for types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
{
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
    ) -> RouterResult<Self> {
        metrics::PAYMENT_CANCEL_COUNT.add(
            &metrics::CONTEXT,
            1,
            &[metrics::request::add_attributes(
                "connector",
                connector.connector_name.to_string(),
            )],
        );

        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::Void,
            types::PaymentsCancelData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &self,
            call_connector_action,
            connector_request,
        )
        .await
        .to_payment_failed_response()?;

        Ok(resp)
    }

    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

    async fn build_flow_specific_connector_request(
        &mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        let request = match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::Void,
                    types::PaymentsCancelData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                connector_integration
                    .build_request(self, &state.conf.connectors)
                    .to_payment_failed_response()?
            }
            _ => None,
        };

        Ok((request, true))
    }
}
