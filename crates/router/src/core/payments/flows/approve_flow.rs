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
impl
    ConstructFlowSpecificData<api::Approve, types::PaymentsApproveData, types::PaymentsResponseData>
    for PaymentData<api::Approve>
{
        /// This method takes in various parameters including the application state, connector ID, merchant account, key store, customer, and merchant connector account. It then constructs payment router data using the given parameters and returns a result of type `RouterResult<types::PaymentsApproveRouterData>`.
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> RouterResult<types::PaymentsApproveRouterData> {
        Box::pin(transformers::construct_payment_router_data::<
            api::Approve,
            types::PaymentsApproveData,
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
impl Feature<api::Approve, types::PaymentsApproveData>
    for types::RouterData<api::Approve, types::PaymentsApproveData, types::PaymentsResponseData>
{
        /// Asynchronously decides on the flows to be executed based on the provided state, connector data, customer information, connector action, merchant account, connector request, and merchant key store. Returns an error response with a "NotImplemented" message indicating that the flow is not supported. 
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

        /// Asynchronously adds an access token for the given merchant account using the provided connector data and application state.
    ///
    /// # Arguments
    ///
    /// * `state` - The application state
    /// * `connector` - The connector data
    /// * `merchant_account` - The merchant account for which the access token needs to be added
    ///
    /// # Returns
    ///
    /// The result of adding the access token, wrapped in a `RouterResult`
    ///
    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

        /// Asynchronously builds a specific connector request for a flow, based on the provided connector data and call connector action.
    ///
    /// # Arguments
    ///
    /// * `_state` - The application state
    /// * `_connector` - The connector data
    /// * `_call_connector_action` - The call connector action
    ///
    /// # Returns
    ///
    /// A `RouterResult` containing a tuple with an optional `services::Request` and a boolean indicating success
    ///
    /// # Errors
    ///
    /// Returns an `ApiErrorResponse` with a `NotImplemented` error and a message indicating that the flow is not supported
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
