use common_utils::ext_traits::{OptionExt, ValueExt};
use diesel_models::subscription::Subscription;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    merchant_context::MerchantContext,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    subscription::ClientSecret,
};

use crate::{consts, core::payments as payments_core, db::errors, routes::SessionState};

pub const SUBSCRIPTION_CONNECTOR_ID: &str = "DefaultSubscriptionConnectorId";
pub const SUBSCRIPTION_PAYMENT_ID: &str = "DefaultSubscriptionPaymentId";

pub struct SubscriptionHandler {
    state: SessionState,
    merchant_context: MerchantContext,
}

impl SubscriptionHandler {
    pub fn new(state: SessionState, merchant_context: MerchantContext) -> Self {
        Self {
            state,
            merchant_context,
        }
    }

    pub async fn find_and_validate_subscription(
        &self,
        client_secret: &ClientSecret,
    ) -> errors::RouterResult<Subscription> {
        let subscription_id = client_secret.get_subscription_id()?;

        let subscription = self
            .state
            .store
            .find_by_merchant_id_subscription_id(
                self.merchant_context.get_merchant_account().get_id(),
                subscription_id.to_string(),
            )
            .await
            .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                message: format!("Subscription not found for id: {subscription_id}"),
            })
            .attach_printable("Unable to find subscription")?;

        self.validate_client_secret(client_secret, &subscription)?;

        Ok(subscription)
    }

    pub async fn get_billing_handler(
        &self,
        subscription: &Subscription,
    ) -> errors::RouterResult<BillingHandler> {
        let mca_id = subscription.get_merchant_connector_id().change_context(
            errors::ApiErrorResponse::GenericNotFoundError {
                message: "merchant_connector_id not found".to_string(),
            },
        )?;

        let billing_processor_mca = self
            .state
            .store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                &(&self.state).into(),
                self.merchant_context.get_merchant_account().get_id(),
                mca_id,
                self.merchant_context.get_merchant_key_store(),
            )
            .await
            .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: mca_id.get_string_repr().to_string(),
            })?;

        let auth_type: ConnectorAuthType =
            super::payments::helpers::MerchantConnectorAccountType::DbVal(Box::new(
                billing_processor_mca.clone(),
            ))
            .get_connector_account_details()
            .parse_value("ConnectorAuthType")
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let connector_data = crate::types::api::ConnectorData::get_connector_by_name(
            &self.state.conf.connectors,
            &billing_processor_mca.connector_name,
            crate::types::api::GetToken::Connector,
            Some(billing_processor_mca.get_id()),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Invalid connector name received in billing merchant connector account",
        )?;

        Ok(BillingHandler {
            subscription: subscription.clone(),
            connector_name: billing_processor_mca.connector_name.clone(),
            auth_type,
            connector_data,
            connector_metadata: billing_processor_mca.metadata.clone(),
        })
    }

    /// Tail helper for secret + expiry validation
    fn validate_client_secret(
        &self,
        client_secret: &ClientSecret,
        subscription: &Subscription,
    ) -> errors::CustomResult<(), errors::ApiErrorResponse> {
        let stored_client_secret = subscription
            .client_secret
            .clone()
            .get_required_value("client_secret")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "client_secret",
            })
            .attach_printable("client secret not found in db")?;

        if client_secret.to_string() != stored_client_secret {
            Err(errors::ApiErrorResponse::ClientSecretInvalid.into())
        } else {
            let current_timestamp = common_utils::date_time::now();
            let session_expiry = subscription
                .created_at
                .saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY));

            if current_timestamp > session_expiry {
                Err(errors::ApiErrorResponse::ClientSecretExpired.into())
            } else {
                Ok(())
            }
        }
    }
}

pub struct BillingHandler {
    pub subscription: Subscription,
    pub connector_name: String,
    pub auth_type: ConnectorAuthType,
    pub connector_data: crate::types::api::ConnectorData,
    pub connector_metadata: Option<common_utils::pii::SecretSerdeValue>,
}

#[allow(clippy::todo)]
impl BillingHandler {
    pub async fn get_subscription_plans(
        &self,
        state: &SessionState,
    ) -> errors::RouterResult<
        hyperswitch_domain_models::router_response_types::subscriptions::GetSubscriptionPlansResponse,
    >{
        let get_plans_request =
            hyperswitch_domain_models::router_request_types::subscriptions::GetSubscriptionPlansRequest::default();

        let router_data = self.build_router_data::<
            hyperswitch_domain_models::router_flow_types::subscriptions::GetSubscriptionPlans,
            hyperswitch_domain_models::router_request_types::subscriptions::GetSubscriptionPlansRequest,
            hyperswitch_domain_models::router_response_types::subscriptions::GetSubscriptionPlansResponse,
        >(state, get_plans_request)?;

        let connector_integration = self.connector_data.connector.get_connector_integration();

        let response = self
            .call_connector(
                state,
                router_data,
                "get subscription plans",
                connector_integration,
            )
            .await?;

        match response {
            Ok(resp) => Ok(resp),
            Err(err) => Err(errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: self.connector_name.clone(),
                status_code: err.status_code,
                reason: err.reason,
            }
            .into()),
        }
    }

    async fn call_connector<F, ResourceCommonData, Req, Resp>(
        &self,
        state: &SessionState,
        router_data: RouterData<F, Req, Resp>,
        operation_name: &str,
        connector_integration: hyperswitch_interfaces::connector_integration_interface::BoxedConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp>,
    ) -> errors::RouterResult<Result<Resp, ErrorResponse>>
    where
        F: Clone + std::fmt::Debug + 'static,
        Req: Clone + std::fmt::Debug + 'static,
        Resp: Clone + std::fmt::Debug + 'static,
        ResourceCommonData:
            hyperswitch_interfaces::connector_integration_interface::RouterDataConversion<
                    F,
                    Req,
                    Resp,
                > + Clone
                + 'static,
    {
        let router_resp = crate::services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data,
            payments_core::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(format!(
            "Failed while calling {operation_name} at billing processor"
        ))?;

        Ok(router_resp.response)
    }

    fn build_router_data<F, Req, Resp>(
        &self,
        state: &SessionState,
        req: Req,
    ) -> errors::RouterResult<RouterData<F, Req, Resp>> {
        Ok(RouterData {
            flow: std::marker::PhantomData,
            merchant_id: self.subscription.merchant_id.to_owned(),
            customer_id: Some(self.subscription.customer_id.to_owned()),
            connector_customer: None,
            connector: self.connector_name.clone(),
            payment_id: SUBSCRIPTION_PAYMENT_ID.to_string(),
            tenant_id: state.tenant.tenant_id.clone(),
            attempt_id: SUBSCRIPTION_PAYMENT_ID.to_owned(),
            status: common_enums::AttemptStatus::default(),
            payment_method: common_enums::PaymentMethod::default(),
            connector_auth_type: self.auth_type.clone(),
            description: None,
            address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
            auth_type: common_enums::AuthenticationType::default(),
            connector_meta_data: self.connector_metadata.clone(),
            connector_wallets_details: None,
            amount_captured: None,
            minor_amount_captured: None,
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_token: None,
            recurring_mandate_payment_data: None,
            preprocessing_id: None,
            payment_method_balance: None,
            connector_api_version: None,
            request: req,
            response: Err(ErrorResponse::default()),
            connector_request_reference_id: SUBSCRIPTION_CONNECTOR_ID.to_owned(),
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            test_mode: None,
            connector_http_status_code: None,
            external_latency: None,
            apple_pay_flow: None,
            frm_metadata: None,
            dispute_id: None,
            refund_id: None,
            payment_method_status: None,
            connector_response: None,
            integrity_check: Ok(()),
            additional_merchant_data: None,
            header_payload: None,
            connector_mandate_request_reference_id: None,
            authentication_id: None,
            psd2_sca_exemption_type: None,
            raw_connector_response: None,
            is_payment_id_from_merchant: None,
            l2_l3_data: None,
            minor_amount_capturable: None,
        })
    }
}

