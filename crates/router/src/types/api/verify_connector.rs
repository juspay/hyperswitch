pub mod paypal;
pub mod stripe;
use error_stack::ResultExt;

use crate::{
    consts,
    core::errors,
    services::{
        self,
        connector_integration_interface::{BoxedConnectorIntegrationInterface, ConnectorEnum},
    },
    types::{self, api, api::ConnectorCommon, domain, storage::enums as storage_enums},
    SessionState,
};

#[derive(Clone)]
pub struct VerifyConnectorData {
    pub connector: ConnectorEnum,
    pub connector_auth: types::ConnectorAuthType,
    pub card_details: domain::Card,
}

impl VerifyConnectorData {
    fn get_payment_authorize_data(&self) -> types::PaymentsAuthorizeData {
        types::PaymentsAuthorizeData {
            payment_method_data: domain::PaymentMethodData::Card(self.card_details.clone()),
            email: None,
            customer_name: None,
            amount: 1000,
            minor_amount: common_utils::types::MinorUnit::new(1000),
            confirm: true,
            order_tax_amount: None,
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
            authentication_data: None,
            customer_acceptance: None,
            split_payments: None,
            merchant_order_reference_id: None,
            integrity_object: None,
            additional_payment_method_data: None,
            shipping_cost: None,
            merchant_account_id: None,
            merchant_config_currency: None,
        }
    }

    fn get_router_data<F, R1, R2>(
        &self,
        state: &SessionState,
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
            attempt_id: attempt_id.clone(),
            description: None,
            customer_id: None,
            tenant_id: state.tenant.tenant_id.clone(),
            merchant_id: common_utils::id_type::MerchantId::default(),
            reference_id: None,
            access_token,
            session_token: None,
            payment_method: storage_enums::PaymentMethod::Card,
            amount_captured: None,
            minor_amount_captured: None,
            preprocessing_id: None,
            connector_customer: None,
            connector_auth_type: self.connector_auth.clone(),
            connector_meta_data: None,
            connector_wallets_details: None,
            payment_method_token: None,
            connector_api_version: None,
            recurring_mandate_payment_data: None,
            payment_method_status: None,
            connector_request_reference_id: attempt_id,
            address: types::PaymentAddress::new(None, None, None, None),
            payment_id: common_utils::id_type::PaymentId::default()
                .get_string_repr()
                .to_owned(),
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
            connector_response: None,
            integrity_check: Ok(()),
            additional_merchant_data: None,
            header_payload: None,
            connector_mandate_request_reference_id: None,
            authentication_id: None,
            psd2_sca_exemption_type: None,
        }
    }
}

#[async_trait::async_trait]
pub trait VerifyConnector {
    async fn verify(
        state: &SessionState,
        connector_data: VerifyConnectorData,
    ) -> errors::RouterResponse<()> {
        let authorize_data = connector_data.get_payment_authorize_data();
        let access_token = Self::get_access_token(state, connector_data.clone()).await?;
        let router_data = connector_data.get_router_data(state, authorize_data, access_token);

        let request = connector_data
            .connector
            .get_connector_integration()
            .build_request(&router_data, &state.conf.connectors)
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "Payment request cannot be built".to_string(),
            })?
            .ok_or(errors::ApiErrorResponse::InternalServerError)?;

        let response =
            services::call_connector_api(&state.to_owned(), request, "verify_connector_request")
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?;

        match response {
            Ok(_) => Ok(services::ApplicationResponse::StatusOk),
            Err(error_response) => {
                Self::handle_payment_error_response::<
                    api::Authorize,
                    types::PaymentFlowData,
                    types::PaymentsAuthorizeData,
                    types::PaymentsResponseData,
                >(
                    connector_data.connector.get_connector_integration(),
                    error_response,
                )
                .await
            }
        }
    }

    async fn get_access_token(
        _state: &SessionState,
        _connector_data: VerifyConnectorData,
    ) -> errors::CustomResult<Option<types::AccessToken>, errors::ApiErrorResponse> {
        // AccessToken is None for the connectors without the AccessToken Flow.
        // If a connector has that, then it should override this implementation.
        Ok(None)
    }

    async fn handle_payment_error_response<F, ResourceCommonData, Req, Resp>(
        // connector: &(dyn api::Connector + Sync),
        connector: BoxedConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp>,
        error_response: types::Response,
    ) -> errors::RouterResponse<()> {
        let error = connector
            .get_error_response(error_response, None)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: error.reason.unwrap_or(error.message),
        }
        .into())
    }

    async fn handle_access_token_error_response<F, ResourceCommonData, Req, Resp>(
        connector: BoxedConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp>,
        error_response: types::Response,
    ) -> errors::RouterResult<Option<types::AccessToken>> {
        let error = connector
            .get_error_response(error_response, None)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: error.reason.unwrap_or(error.message),
        }
        .into())
    }
}
