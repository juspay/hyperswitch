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

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::UpdateMetadata,
        types::PaymentsUpdateMetadataData,
        types::PaymentsResponseData,
    > for PaymentData<api::UpdateMetadata>
{
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
    ) -> RouterResult<types::PaymentsUpdateMetadataRouterData> {
        Box::pin(
            transformers::construct_payment_router_data_for_update_metadata(
                state,
                self.clone(),
                connector_id,
                platform,
                customer,
                merchant_connector_account,
                merchant_recipient_data,
                header_payload,
            ),
        )
        .await
    }

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
    ) -> RouterResult<types::PaymentsUpdateMetadataRouterData> {
        todo!()
    }
}

#[async_trait]
impl Feature<api::UpdateMetadata, types::PaymentsUpdateMetadataData>
    for types::RouterData<
        api::UpdateMetadata,
        types::PaymentsUpdateMetadataData,
        types::PaymentsResponseData,
    >
{
    async fn decide_flows<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        _business_profile: &domain::Profile,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        return_raw_connector_response: Option<bool>,
        _gateway_context: payments::gateway::context::RouterGatewayContext,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::UpdateMetadata,
            types::PaymentsUpdateMetadataData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &self,
            call_connector_action,
            connector_request,
            return_raw_connector_response,
        )
        .await
        .to_payment_failed_response()?;
        Ok(resp)
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
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        let request = match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                    api::UpdateMetadata,
                    types::PaymentsUpdateMetadataData,
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
