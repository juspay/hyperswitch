use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ApiErrorResponse, NotImplementedMessage, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
    },
    routes::SessionState,
    services,
    types::{self, api, domain},
};

#[async_trait]
impl ConstructFlowSpecificData<api::Reject, types::PaymentsRejectData, types::PaymentsResponseData>
    for PaymentData<api::Reject>
{
    #[cfg(feature = "v1")]
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        merchant_context: &domain::MerchantContext,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<types::PaymentsRejectRouterData> {
        Box::pin(transformers::construct_payment_router_data::<
            api::Reject,
            types::PaymentsRejectData,
        >(
            state,
            self.clone(),
            connector_id,
            merchant_context,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
        ))
        .await
    }

    #[cfg(feature = "v2")]
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        merchant_context: &domain::MerchantContext,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<types::PaymentsRejectRouterData> {
        todo!()
    }
}

#[async_trait]
impl Feature<api::Reject, types::PaymentsRejectData>
    for types::RouterData<api::Reject, types::PaymentsRejectData, types::PaymentsResponseData>
{
    async fn decide_flows<'a>(
        self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _call_connector_action: payments::CallConnectorAction,
        _connector_request: Option<services::Request>,
        _business_profile: &domain::Profile,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        _return_raw_connector_response: Option<bool>,
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
        merchant_context: &domain::MerchantContext,
        creds_identifier: Option<&str>,
    ) -> RouterResult<types::AddAccessTokenResult> {
        Box::pin(access_token::add_access_token(
            state,
            connector,
            merchant_context,
            self,
            creds_identifier,
        ))
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
