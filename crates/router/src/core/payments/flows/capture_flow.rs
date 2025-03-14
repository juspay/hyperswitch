use async_trait::async_trait;

use super::ConstructFlowSpecificData;
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, Feature, PaymentData},
    },
    routes::SessionState,
    services,
    types::{self, api, domain},
};

#[cfg(feature = "v1")]
#[async_trait]
impl
    ConstructFlowSpecificData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for PaymentData<api::Capture>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
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
            merchant_recipient_data,
            header_payload,
        ))
        .await
    }

    async fn get_merchant_recipient_data<'a>(
        &self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _merchant_connector_account: &helpers::MerchantConnectorAccountType,
        _connector: &api::ConnectorData,
    ) -> RouterResult<Option<types::MerchantRecipientData>> {
        Ok(None)
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl
    ConstructFlowSpecificData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for hyperswitch_domain_models::payments::PaymentCaptureData<api::Capture>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccount,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<
        types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>,
    > {
        Box::pin(transformers::construct_payment_router_data_for_capture(
            state,
            self.clone(),
            connector_id,
            merchant_account,
            key_store,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
        ))
        .await
    }

    async fn get_merchant_recipient_data<'a>(
        &self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _merchant_connector_account: &helpers::MerchantConnectorAccountType,
        _connector: &api::ConnectorData,
    ) -> RouterResult<Option<types::MerchantRecipientData>> {
        Ok(None)
    }
}

#[async_trait]
impl Feature<api::Capture, types::PaymentsCaptureData>
    for types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
{
    async fn decide_flows<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        _business_profile: &domain::Profile,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let mut new_router_data = services::execute_connector_processing_step(
            state,
            connector_integration,
            &self,
            call_connector_action,
            connector_request,
        )
        .await
        .to_payment_failed_response()?;

        // Initiating Integrity check
        let integrity_result = helpers::check_integrity_based_on_flow(
            &new_router_data.request,
            &new_router_data.response,
        );
        new_router_data.integrity_check = integrity_result;

        Ok(new_router_data)
    }

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        creds_identifier: Option<&str>,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self, creds_identifier)
            .await
    }

    async fn build_flow_specific_connector_request(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        let request = match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
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
