use async_trait::async_trait;
use common_enums as enums;
use common_types::payments as common_payments_types;
use error_stack::ResultExt;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::PaymentConfirmData;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    router_data::RouterData,
    router_flow_types::{PostAuthenticate, PreAuthenticate},
    router_request_types::PaymentsAuthorizeData,
};
use masking::{ExposeInterface, Secret};
use unified_connector_service_client::payments as payments_grpc;

// use router_env::tracing::Instrument;
use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        mandate,
        payments::{
            self, access_token, customers, helpers, tokenization, transformers, PaymentData,
        },
        unified_connector_service::{
            build_unified_connector_service_auth_metadata,
            handle_unified_connector_service_response_for_payment_authorize,
        },
    },
    logger,
    routes::{metrics, SessionState},
    services::{self, api::ConnectorValidation},
    types::{
        self, api, domain,
        transformers::{ForeignFrom, ForeignTryFrom},
    },
    utils::OptionExt,
};

#[async_trait]
impl Feature<PostAuthenticate, types::PaymentsPostAuthenticateData>
    for RouterData<
        PostAuthenticate,
        types::PaymentsPostAuthenticateData,
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
        return_raw_connector_response: Option<bool>,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            PostAuthenticate,
            types::PaymentsPostAuthenticateData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let postauth_router_data = services::execute_connector_processing_step(
            state,
            connector_integration,
            &self,
            call_connector_action.clone(),
            connector_request,
            return_raw_connector_response,
        )
        .await
        .to_payment_failed_response()?;

        Ok(postauth_router_data)
    }

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        merchant_context: &domain::MerchantContext,
        creds_identifier: Option<&str>,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_context, self, creds_identifier)
            .await
    }

    async fn build_flow_specific_connector_request(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                // connector
                //     .connector
                //     .validate_connector_against_payment_request(
                //         self.request.capture_method,
                //         self.payment_method,
                //         self.request.payment_method_type,
                //     )
                //     .to_payment_failed_response()?;

                let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                    PostAuthenticate,
                    types::PaymentsPostAuthenticateData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                metrics::EXECUTE_PRETASK_COUNT.add(
                    1,
                    router_env::metric_attributes!(
                        ("connector", connector.connector_name.to_string()),
                        ("flow", format!("{:?}", PostAuthenticate)),
                    ),
                );

                logger::debug!(completed_pre_tasks=?true);

                Ok((
                    connector_integration
                        .build_request(self, &state.conf.connectors)
                        .to_payment_failed_response()?,
                    true,
                ))
            }
            _ => Ok((None, true)),
        }
    }

    async fn call_unified_connector_service<'a>(
        &mut self,
        state: &SessionState,
        #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        merchant_context: &domain::MerchantContext,
    ) -> RouterResult<()> {
        todo!()
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl
    ConstructFlowSpecificData<
        PostAuthenticate,
        types::PaymentsPostAuthenticateData,
        types::PaymentsResponseData,
    > for PaymentConfirmData<PostAuthenticate>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        merchant_context: &domain::MerchantContext,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<
        RouterData<
            PostAuthenticate,
            types::PaymentsPostAuthenticateData,
            types::PaymentsResponseData,
        >,
    > {
        Box::pin(
            transformers::construct_payment_router_data_for_post_authenticate(
                state,
                self.clone(),
                connector_id,
                merchant_context,
                customer,
                merchant_connector_account,
                merchant_recipient_data,
                header_payload,
            ),
        )
        .await
    }
}
