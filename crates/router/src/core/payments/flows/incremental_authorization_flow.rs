use async_trait::async_trait;

use super::ConstructFlowSpecificData;
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, Feature, PaymentData},
    },
    routes::AppState,
    services,
    types::{self, api, domain},
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::IncrementalAuthorization,
        types::PaymentsIncrementalAuthorizationData,
        types::PaymentsResponseData,
    > for PaymentData<api::IncrementalAuthorization>
{
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> RouterResult<types::PaymentsIncrementalAuthorizationRouterData> {
        Box::pin(transformers::construct_payment_router_data::<
            api::IncrementalAuthorization,
            types::PaymentsIncrementalAuthorizationData,
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
impl Feature<api::IncrementalAuthorization, types::PaymentsIncrementalAuthorizationData>
    for hyperswitch_domain_models::router_data::RouterData<
        api::IncrementalAuthorization,
        types::PaymentsIncrementalAuthorizationData,
        types::PaymentsResponseData,
    >
{
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::IncrementalAuthorization,
            types::PaymentsIncrementalAuthorizationData,
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
                    api::IncrementalAuthorization,
                    types::PaymentsIncrementalAuthorizationData,
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
