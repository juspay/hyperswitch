use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, access_token, transformers, PaymentData},
    },
    routes, services,
    types::{self, api, storage},
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::PaymentMethodToken,
        types::TokenizationData,
        types::PaymentsResponseData,
    > for PaymentData<api::PaymentMethodToken>
{
    async fn construct_router_data<'a>(
        &self,
        state: &routes::AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::TokenizationRouterData> {
        transformers::construct_payment_router_data::<
            api::PaymentMethodToken,
            types::TokenizationData,
        >(state, self.clone(), connector_id, merchant_account)
        .await
    }
}

#[async_trait]
impl Feature<api::PaymentMethodToken, types::TokenizationData> for types::TokenizationRouterData {
    async fn decide_flows<'a>(
        self,
        state: &routes::AppState,
        connector: &api::ConnectorData,
        customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<Self> {
        self.decide_flow(
            state,
            connector,
            customer,
            Some(true),
            call_connector_action,
            merchant_account,
        )
        .await
    }

    async fn add_access_token<'a>(
        &self,
        state: &routes::AppState,
        connector: &api::ConnectorData,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }
}

impl types::TokenizationRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a routes::AppState,
        connector: &api::ConnectorData,
        _customer: &Option<storage::Customer>,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
        _merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::PaymentMethodToken,
            types::TokenizationData,
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
