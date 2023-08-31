use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{api_error_response::NotImplementedMessage, ApiErrorResponse, RouterResult},
        payments::{self, access_token, transformers, PaymentData},
    },
    routes::AppState,
    services,
    types::{self, api, domain},
};

#[async_trait]
impl
    ConstructFlowSpecificData<api::Decline, types::PaymentsDeclineData, types::PaymentsResponseData>
    for PaymentData<api::Decline>
{
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
    ) -> RouterResult<types::PaymentsDeclineRouterData> {
        transformers::construct_payment_router_data::<api::Decline, types::PaymentsDeclineData>(
            state,
            self.clone(),
            connector_id,
            merchant_account,
            key_store,
            customer,
        )
        .await
    }
}

#[async_trait]
impl Feature<api::Decline, types::PaymentsDeclineData>
    for types::RouterData<api::Decline, types::PaymentsDeclineData, types::PaymentsResponseData>
{
    async fn decide_flows<'a>(
        self,
        _state: &AppState,
        _connector: &api::ConnectorData,
        _customer: &Option<domain::Customer>,
        _call_connector_action: payments::CallConnectorAction,
        _merchant_account: &domain::MerchantAccount,
        _connector_request: Option<services::Request>,
    ) -> RouterResult<Self> {
        Err(ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason("Flow not supported".to_string()),
        }
        .into())
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
