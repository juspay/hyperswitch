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
        /// Asynchronously constructs router data for payments cancellation by calling the `construct_payment_router_data` method with the provided parameters and awaits for the result.
    ///
    /// # Arguments
    ///
    /// * `state` - A reference to the application state.
    /// * `connector_id` - The ID of the connector.
    /// * `merchant_account` - A reference to the merchant account.
    /// * `key_store` - A reference to the merchant key store.
    /// * `customer` - An optional reference to the customer.
    /// * `merchant_connector_account` - The merchant connector account type.
    ///
    /// # Returns
    ///
    /// The router result containing the constructed payments cancellation router data.
    ///
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
        /// This method is responsible for deciding the flows of the payment system. It updates the payment cancel count metric, gets the connector integration, and executes the connector processing step. It then awaits the result and returns a payment failed response.
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
        _customer: &Option<domain::Customer>,
        call_connector_action: payments::CallConnectorAction,
        _merchant_account: &domain::MerchantAccount,
        connector_request: Option<services::Request>,
        _key_store: &domain::MerchantKeyStore,
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

        /// Asynchronously adds an access token to the given merchant account using the provided connector data and application state.
    /// 
    /// # Arguments
    /// 
    /// * `state` - The application state containing necessary data and configurations.
    /// * `connector` - The connector data used to authenticate and authorize the request.
    /// * `merchant_account` - The merchant account to which the access token will be added.
    /// 
    /// # Returns
    /// 
    /// A `RouterResult` containing the result of adding the access token to the merchant account.
    /// 
    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }


        /// Asynchronously builds a specific connector request based on the provided state, connector data, and call connector action. 
    /// Returns a tuple containing the optional request and a boolean indicating whether the request was successfully built.
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
