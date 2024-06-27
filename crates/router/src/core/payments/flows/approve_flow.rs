use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ApiErrorResponse, NotImplementedMessage, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
    },
    routes::SessionState,
    services,
    types::{self, api, domain, storage},
};

#[async_trait]
impl
    ConstructFlowSpecificData<api::Approve, types::PaymentsApproveData, types::PaymentsResponseData>
    for PaymentData<api::Approve>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
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
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _call_connector_action: payments::CallConnectorAction,
        _connector_request: Option<services::Request>,
        _business_profile: &storage::business_profile::BusinessProfile,
        _header_payload: api_models::payments::HeaderPayload,
    ) -> RouterResult<Self> {
        Err(ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason("Flow not supported".to_string()),
        }
        .into())
    }

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        creds_identifier: Option<&String>,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self, creds_identifier)
            .await
    }

    async fn build_flow_specific_connector_request(
        &mut self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        Err(ApiErrorResponse::NotImplemented {
            message: NotImplementedMessage::Reason("Flow not supported".to_string()),
        }
        .into())
    }
}
