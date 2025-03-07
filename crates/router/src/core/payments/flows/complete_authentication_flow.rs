use async_trait::async_trait;
use masking::ExposeInterface;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ApiErrorResponse, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
    },
    routes::{metrics, SessionState},
    services,
    types::{self, api, domain, transformers::ForeignTryFrom},
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::CompleteAuthentication,
        types::CompleteAuthenticationData,
        types::PaymentsResponseData,
    > for PaymentData<api::CompleteAuthorize>
{
    #[cfg(feature = "v1")]
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
    ) -> RouterResult<
        types::RouterData<
            api::CompleteAuthorize,
            types::CompleteAuthenticationData,
            types::PaymentsResponseData,
        >,
    > {
        Box::pin(transformers::construct_payment_router_data::<
            api::CompleteAuthentication,
            types::CompleteAuthenticationData,
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

    #[cfg(feature = "v2")]
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
        types::RouterData<
            api::CompleteAuthentication,
            types::CompleteAuthenticationData,
            types::PaymentsResponseData,
        >,
    > {
        todo!()
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
impl Feature<api::CompleteAuthorize, types::CompleteAuthenticationData>
    for types::RouterData<
        api::CompleteAuthentication,
        types::CompleteAuthenticationData,
        types::PaymentsResponseData,
    >
{
    async fn decide_flows<'a>(
        mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        business_profile: &domain::Profile,
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::CompleteAuthentication,
            types::CompleteAuthenticationData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let mut complete_authentication_router_data = services::execute_connector_processing_step(
            state,
            connector_integration,
            &self,
            call_connector_action.clone(),
            connector_request,
        )
        .await
        .to_payment_failed_response()?;
    
        match complete_authentication_router_data.response.clone() {
            Err(_) => Ok(complete_authentication_router_data),
            Ok(complete_authentication_response) => {
                // Check if the Capture API should be called based on the connector and other parameters
                if super::should_initiate_complete_authorize(
                    state,
                    &connector.connector_name,
                    &complete_authentication_response,
                ) {
                    complete_authentication_router_data = Box::pin(process_complete_authorization(
                        complete_authentication_router_data,
                        complete_authentication_response,
                        state,
                        connector,
                        call_connector_action.clone(),
                        business_profile,
                        header_payload,
                    ))
                    .await?;
                }
                Ok(complete_authentication_router_data)
            }
        }
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
                    api::CompleteAuthentication,
                    types::CompleteAuthenticationData,
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

impl<F>
    ForeignTryFrom<types::RouterData<F, types::CompleteAuthenticationData, types::PaymentsResponseData>>
    for types::CompleteAuthorizeData
{
    type Error = error_stack::Report<ApiErrorResponse>;

    fn foreign_try_from(
        item: types::RouterData<F, types::CompleteAuthenticationData, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = item
            .response
            .map_err(|err| ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: item.connector.clone().to_string(),
                status_code: err.status_code,
                reason: err.reason,
            })?;

        Ok(Self {
            currency: item.request.currency,
            connector_transaction_id: item.request.connector_transaction_id,
            connector_meta: types::PaymentsResponseData::get_connector_metadata(&response)
                .map(|secret| secret.expose()),
            browser_info: None,
            metadata: None,
            capture_method: item.request.capture_method,
            payment_method_data: item.request.payment_method_data,
            amount: item.request.amount,
            email: item.get_billing_email(),
            confirm: item.request.confirm,
            statement_descriptor_suffix: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            redirect_response: None,
            complete_authorize_url: item.request.complete_authorize_url,
            customer_acceptance: None,
            minor_amount: item.request.minor_amount,
        })
    }
}

async fn process_complete_authorization(
    mut router_data: types::RouterData<
        api::CompleteAuthentication,
        types::CompleteAuthenticationData,
        types::PaymentsResponseData,
    >,
    complete_authorize_response: types::PaymentsResponseData,
    state: &SessionState,
    connector: &api::ConnectorData,
    call_connector_action: payments::CallConnectorAction,
    business_profile: &domain::Profile,
    header_payload: hyperswitch_domain_models::payments::HeaderPayload,
) -> RouterResult<
    types::RouterData<
        api::CompleteAuthorize,
        types::CompleteAuthenticationData,
        types::PaymentsResponseData,
    >,
> {
    // Convert RouterData into Capture RouterData
    let capture_router_data = helpers::router_data_type_conversion(
        router_data.clone(),
        types::CompleteAuthorizeData::foreign_try_from(router_data.clone())?,
        Err(types::ErrorResponse::default()),
    );

    // Call capture request
    let post_capture_router_data = super::call_complete_authorization_request(
        capture_router_data,
        state,
        connector,
        call_connector_action,
        business_profile,
        header_payload,
    )
    .await;

    // Process capture response
    let (updated_status, updated_response) =
        super::handle_post_complete_authorize_response(complete_authorize_response, post_capture_router_data)?;

    router_data.status = updated_status;
    router_data.response = Ok(updated_response);
    Ok(router_data)
}
