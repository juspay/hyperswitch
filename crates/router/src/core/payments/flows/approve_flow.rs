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
    async fn decide_flows<'a>(
        self,
        _state: &AppState,
        _connector: &api::ConnectorData,
        _call_connector_action: payments::CallConnectorAction,
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
