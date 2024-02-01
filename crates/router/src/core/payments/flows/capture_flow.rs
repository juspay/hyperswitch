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
    ConstructFlowSpecificData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for PaymentData<api::Capture>
{
        /// Constructs router data for capturing payments.
    ///
    /// This method takes in the necessary data to construct router data for capturing payments, including the application state, connector ID, merchant account, key store, customer information, and merchant connector account type. It then utilizes the `transformers::construct_payment_router_data` function to transform the input data into a `RouterResult` containing `types::PaymentsCaptureRouterData`.
    ///
    /// # Arguments
    ///
    /// * `state` - The application state.
    /// * `connector_id` - The ID of the connector.
    /// * `merchant_account` - The merchant account information.
    /// * `key_store` - The merchant key store information.
    /// * `customer` - An optional customer information.
    /// * `merchant_connector_account` - The merchant connector account type.
    ///
    /// # Returns
    ///
    /// The constructed router data for capturing payments as a `RouterResult` containing `types::PaymentsCaptureRouterData`.
    ///
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> RouterResult<types::PaymentsCaptureRouterData> {
        Box::pin(transformers::construct_payment_router_data::<
            api::Capture,
            types::PaymentsCaptureData,
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
impl Feature<api::Capture, types::PaymentsCaptureData>
    for types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
{
        /// Asynchronously decides the flows for processing a payment transaction by executing the connector integration
    /// 
    /// # Arguments
    /// 
    /// * `state` - The application state
    /// * `connector` - The connector data
    /// * `_customer` - The optional customer data
    /// * `call_connector_action` - The action to be invoked on the connector
    /// * `_merchant_account` - The merchant account data
    /// * `connector_request` - The optional connector request
    /// * `_key_store` - The merchant key store
    /// 
    /// # Returns
    /// 
    /// A `RouterResult` containing the response of the payment processing
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
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::Capture,
            types::PaymentsCaptureData,
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

        /// Asynchronously adds an access token for a merchant account using the given state, connector data, and merchant account information.
    /// 
    /// # Arguments
    /// * `state` - The state of the application
    /// * `connector` - The connector data for the merchant account
    /// * `merchant_account` - The information of the merchant account
    /// 
    /// # Returns
    /// The result of adding the access token, wrapped in a `RouterResult`
    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

        /// Asynchronously builds a flow-specific connector request based on the provided connector data and action.
    ///
    /// # Arguments
    ///
    /// * `state` - The shared application state
    /// * `connector` - The connector data to build the request for
    /// * `call_connector_action` - The action to perform on the connector
    ///
    /// # Returns
    ///
    /// A tuple containing the built request and a boolean indicating whether the request was successfully built
    ///
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
                    api::Capture,
                    types::PaymentsCaptureData,
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
