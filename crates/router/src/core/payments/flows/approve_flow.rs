use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ApiErrorResponse, NotImplementedMessage, RouterResult},
        payments::{
            self, access_token, flows::gateway_context, helpers, transformers, PaymentData,
        },
    },
    routes::SessionState,
    services,
    types::{self, api, domain},
};

#[async_trait]
impl
    ConstructFlowSpecificData<api::Approve, types::PaymentsApproveData, types::PaymentsResponseData>
    for PaymentData<api::Approve>
{
    #[cfg(feature = "v2")]
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        platform: &domain::Platform,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<types::PaymentsApproveRouterData> {
        todo!()
    }

    #[cfg(feature = "v1")]
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        platform: &domain::Platform,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
        _payment_method: Option<common_enums::PaymentMethod>,
        _payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> RouterResult<types::PaymentsApproveRouterData> {
        Box::pin(transformers::construct_payment_router_data::<
            api::Approve,
            types::PaymentsApproveData,
        >(
            state,
            self.clone(),
            connector_id,
            platform,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
            None,
            None,
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
        _business_profile: &domain::Profile,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        _return_raw_connector_response: Option<bool>,
        _gateway_context: gateway_context::RouterGatewayContext,
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
        _platform: &domain::Platform,
        creds_identifier: Option<&str>,
    ) -> RouterResult<types::AddAccessTokenResult> {
        Box::pin(access_token::add_access_token(
            state,
            connector,
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
