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
        /// Asynchronously constructs router data for payments incremental authorization, using the provided state, connector ID, merchant account, key store, customer, and merchant connector account. This method returns a RouterResult containing the constructed payments incremental authorization router data.
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
    for types::RouterData<
        api::IncrementalAuthorization,
        types::PaymentsIncrementalAuthorizationData,
        types::PaymentsResponseData,
    >
{
        /// This method is responsible for deciding the flows of the payment process. It takes in various parameters including the application state, connector data, customer information, connector action, merchant account, connector request, and merchant key store. It then obtains the connector integration, executes the connector processing step, and returns the resulting payment response. 
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

        /// Asynchronously adds an access token for the given merchant account using the provided app state, connector data, and self reference. It returns a RouterResult containing the result of adding the access token.
    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

        /// Asynchronously builds a specific connector request based on the provided connector data and call connector action.
    /// 
    /// # Arguments
    /// 
    /// * `state` - The application state.
    /// * `connector` - The connector data.
    /// * `call_connector_action` - The action to be performed on the connector.
    /// 
    /// # Returns
    /// 
    /// A `RouterResult` containing a tuple with an optional `services::Request` and a boolean. The boolean indicates whether the operation was successful.

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
