pub mod paypal;
pub mod stripe;

use error_stack::{IntoReport, ResultExt};

use crate::{
    consts,
    core::errors,
    services,
    services::ConnectorIntegration,
    types::{self, api, storage::enums as storage_enums},
    AppState,
};

#[derive(Clone, Debug)]
pub struct VerifyConnectorData {
    pub connector: &'static (dyn types::api::Connector + Sync),
    pub connector_auth: types::ConnectorAuthType,
    pub card_details: api::Card,
}

impl VerifyConnectorData {
    fn get_payment_authorize_data(&self) -> types::PaymentsAuthorizeData {
        types::PaymentsAuthorizeData {
            payment_method_data: api::PaymentMethodData::Card(self.card_details.clone()),
            email: None,
            customer_name: None,
            amount: 1000,
            confirm: true,
            currency: storage_enums::Currency::USD,
            metadata: None,
            mandate_id: None,
            webhook_url: None,
            customer_id: None,
            off_session: None,
            browser_info: None,
            session_token: None,
            order_details: None,
            order_category: None,
            capture_method: None,
            enrolled_for_3ds: false,
            router_return_url: None,
            surcharge_details: None,
            setup_future_usage: None,
            payment_experience: None,
            payment_method_type: None,
            statement_descriptor: None,
            setup_mandate_details: None,
            complete_authorize_url: None,
            related_transaction_id: None,
            statement_descriptor_suffix: None,
            request_incremental_authorization: false,
        }
    }

    fn get_router_data<F, R1, R2>(
        &self,
        request_data: R1,
        access_token: Option<types::AccessToken>,
    ) -> types::RouterData<F, R1, R2> {
        let attempt_id =
            common_utils::generate_id_with_default_len(consts::VERIFY_CONNECTOR_ID_PREFIX);
        types::RouterData {
            flow: std::marker::PhantomData,
            status: storage_enums::AttemptStatus::Started,
            request: request_data,
            response: Err(errors::ApiErrorResponse::InternalServerError.into()),
            connector: self.connector.id().to_string(),
            auth_type: storage_enums::AuthenticationType::NoThreeDs,
            test_mode: None,
            return_url: None,
            attempt_id: attempt_id.clone(),
            description: None,
            customer_id: None,
            merchant_id: consts::VERIFY_CONNECTOR_MERCHANT_ID.to_string(),
            reference_id: None,
            access_token,
            session_token: None,
            payment_method: storage_enums::PaymentMethod::Card,
            amount_captured: None,
            preprocessing_id: None,
            payment_method_id: None,
            connector_customer: None,
            connector_auth_type: self.connector_auth.clone(),
            connector_meta_data: None,
            payment_method_token: None,
            connector_api_version: None,
            recurring_mandate_payment_data: None,
            connector_request_reference_id: attempt_id,
            address: types::PaymentAddress {
                shipping: None,
                billing: None,
                payment_method_billing: None,
            },
            payment_id: common_utils::generate_id_with_default_len(
                consts::VERIFY_CONNECTOR_ID_PREFIX,
            ),
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            payment_method_balance: None,
            connector_http_status_code: None,
            external_latency: None,
            apple_pay_flow: None,
            frm_metadata: None,
            refund_id: None,
            dispute_id: None,
        }
    }
}

#[async_trait::async_trait]
pub trait VerifyConnector {
    async fn verify(
        state: &AppState,
        connector_data: VerifyConnectorData,
    ) -> errors::RouterResponse<()> {
        let authorize_data = connector_data.get_payment_authorize_data();
        let access_token = Self::get_access_token(state, connector_data.clone()).await?;
        let router_data = connector_data.get_router_data(authorize_data, access_token);

        let request = connector_data
            .connector
            .build_request(&router_data, &state.conf.connectors)
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "Payment request cannot be built".to_string(),
            })?
            .ok_or(errors::ApiErrorResponse::InternalServerError)?;

        let response = services::call_connector_api(&state.to_owned(), request)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        match response {
            Ok(_) => Ok(services::ApplicationResponse::StatusOk),
            Err(error_response) => {
                Self::handle_payment_error_response::<
                    api::Authorize,
                    types::PaymentsAuthorizeData,
                    types::PaymentsResponseData,
                >(connector_data.connector, error_response)
                .await
            }
        }
    }

    async fn get_access_token(
        _state: &AppState,
        _connector_data: VerifyConnectorData,
    ) -> errors::CustomResult<Option<types::AccessToken>, errors::ApiErrorResponse> {
        // AccessToken is None for the connectors without the AccessToken Flow.
        // If a connector has that, then it should override this implementation.
        Ok(None)
    }

    async fn handle_payment_error_response<F, R1, R2>(
        connector: &(dyn types::api::Connector + Sync),
        error_response: types::Response,
    ) -> errors::RouterResponse<()>
    where
        dyn types::api::Connector + Sync: ConnectorIntegration<F, R1, R2>,
    {
        let error = connector
            .get_error_response(error_response, None)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: error.reason.unwrap_or(error.message),
        })
        .into_report()
    }

    async fn handle_access_token_error_response<F, R1, R2>(
        connector: &(dyn types::api::Connector + Sync),
        error_response: types::Response,
    ) -> errors::RouterResult<Option<types::AccessToken>>
    where
        dyn types::api::Connector + Sync: ConnectorIntegration<F, R1, R2>,
    {
        let error = connector
            .get_error_response(error_response, None)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: error.reason.unwrap_or(error.message),
        })
        .into_report()
    }
}
