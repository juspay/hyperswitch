use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{api_error_response::NotImplementedMessage, ApiErrorResponse, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
    },
    routes::AppState,
    services,
    types::{self, api, domain},
};

#[async_trait]
impl ConstructFlowSpecificData<api::Reject, types::PaymentsRejectData, types::PaymentsResponseData>
    for PaymentData<api::Reject>
{
        /// Asynchronously constructs router data for payment rejection, using the given state, connector ID, merchant account, key store, customer, and merchant connector account. Returns a `RouterResult` containing the constructed payment reject router data.
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> RouterResult<types::PaymentsRejectRouterData> {
        Box::pin(transformers::construct_payment_router_data::<
            api::Reject,
            types::PaymentsRejectData,
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
impl Feature<api::Reject, types::PaymentsRejectData>
    for types::RouterData<api::Reject, types::PaymentsRejectData, types::PaymentsResponseData>
{
        /// Asynchronously decides the flows based on the given input parameters and returns a `RouterResult<Self>`.
    async fn decide_flows<'a>(
        self,
        _state: &AppState,
        _connector: &api::ConnectorData,
        _customer: &Option<domain::Customer>,
        _call_connector_action: payments::CallConnectorAction,
        _merchant_account: &domain::MerchantAccount,
        _connector_request: Option<services::Request>,
        _key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<Self> {
        Err(ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason("Flow not supported".to_string()),
        }
        .into())
    }

        /// Asynchronously adds an access token for a merchant account using the provided state, connector data, and merchant account information.
    ///
    /// # Arguments
    ///
    /// * `state` - The application state containing the necessary resources for adding the access token.
    /// * `connector` - The connector data used to authenticate and authorize the access token.
    /// * `merchant_account` - The merchant account for which the access token is being added.
    ///
    /// # Returns
    ///
    /// A `RouterResult` containing the result of adding the access token.
    ///
    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

        /// Asynchronously builds a specific connector request for the flow.
    ///
    /// # Arguments
    ///
    /// * `state` - The application state
    /// * `connector` - The connector data
    /// * `call_connector_action` - The action to call the connector
    ///
    /// # Returns
    ///
    /// A result containing an optional request and a boolean indicating success
    ///
    /// # Errors
    ///
    /// Returns an `ApiErrorResponse::NotImplemented` if the flow is not supported
    ///
    async fn build_flow_specific_connector_request(
        &mut self,
        _state: &AppState,
        _connector: &api::ConnectorData,
        _call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        Err(ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason("Flow not supported".to_string()),
        }
        .into())
    }
}
