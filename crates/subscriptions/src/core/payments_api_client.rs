use api_models::subscription as subscription_types;
use common_utils::{ext_traits::BytesExt, request as services};
use error_stack::ResultExt;
use hyperswitch_interfaces::api_client as api;

use crate::{core::errors, helpers, state::SubscriptionState as SessionState};

pub struct PaymentsApiClient;

#[derive(Debug, serde::Deserialize)]
pub struct ErrorResponse {
    error: ErrorResponseDetails,
}

#[derive(Debug, serde::Deserialize)]
pub struct ErrorResponseDetails {
    #[serde(rename = "type")]
    error_type: Option<String>,
    code: String,
    message: String,
}

impl PaymentsApiClient {
    fn get_internal_auth_headers(
        state: &SessionState,
        merchant_id: &str,
        profile_id: &str,
    ) -> Vec<(String, masking::Maskable<String>)> {
        vec![
            (
                helpers::X_INTERNAL_API_KEY.to_string(),
                masking::Maskable::Masked(
                    state
                        .conf
                        .internal_merchant_id_profile_id_auth
                        .internal_api_key
                        .clone(),
                ),
            ),
            (
                helpers::X_TENANT_ID.to_string(),
                masking::Maskable::Normal(state.tenant.tenant_id.get_string_repr().to_string()),
            ),
            (
                helpers::X_MERCHANT_ID.to_string(),
                masking::Maskable::Normal(merchant_id.to_string()),
            ),
            (
                helpers::X_PROFILE_ID.to_string(),
                masking::Maskable::Normal(profile_id.to_string()),
            ),
        ]
    }

    /// Generic method to handle payment API calls with different HTTP methods and URL patterns
    async fn make_payment_api_call(
        state: &SessionState,
        method: services::Method,
        url: String,
        request_body: Option<common_utils::request::RequestContent>,
        operation_name: &str,
        merchant_id: &str,
        profile_id: &str,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        let subscription_error = errors::ApiErrorResponse::SubscriptionError {
            operation: operation_name.to_string(),
        };
        let headers = Self::get_internal_auth_headers(state, merchant_id, profile_id);

        let mut request_builder = services::RequestBuilder::new()
            .method(method)
            .url(&url)
            .headers(headers);

        // Add request body only if provided (for POST requests)
        if let Some(body) = request_body {
            request_builder = request_builder.set_body(body);
        }

        let request = request_builder.build();
        let response = api::call_connector_api(state, request, "Subscription Payments")
            .await
            .change_context(subscription_error.clone())?;

        match response {
            Ok(res) => {
                let api_response: subscription_types::PaymentResponseData = res
                    .response
                    .parse_struct(std::any::type_name::<subscription_types::PaymentResponseData>())
                    .change_context(subscription_error)?;
                Ok(api_response)
            }
            Err(err) => {
                let error_response: ErrorResponse = err
                    .response
                    .parse_struct(std::any::type_name::<ErrorResponse>())
                    .change_context(subscription_error)?;
                Err(errors::ApiErrorResponse::ExternalConnectorError {
                    code: error_response.error.code,
                    message: error_response.error.message,
                    connector: "payments_microservice".to_string(),
                    status_code: err.status_code,
                    reason: error_response.error.error_type,
                }
                .into())
            }
        }
    }

    pub async fn create_cit_payment(
        state: &SessionState,
        request: subscription_types::CreatePaymentsRequestData,
        merchant_id: &str,
        profile_id: &str,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        let base_url = &state.conf.internal_services.payments_base_url;
        let url = format!("{}/payments", base_url);

        Self::make_payment_api_call(
            state,
            services::Method::Post,
            url,
            Some(common_utils::request::RequestContent::Json(Box::new(
                request,
            ))),
            "Create Payment",
            merchant_id,
            profile_id,
        )
        .await
    }

    pub async fn create_and_confirm_payment(
        state: &SessionState,
        request: subscription_types::CreateAndConfirmPaymentsRequestData,
        merchant_id: &str,
        profile_id: &str,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        let base_url = &state.conf.internal_services.payments_base_url;
        let url = format!("{}/payments", base_url);

        Self::make_payment_api_call(
            state,
            services::Method::Post,
            url,
            Some(common_utils::request::RequestContent::Json(Box::new(
                request,
            ))),
            "Create And Confirm Payment",
            merchant_id,
            profile_id,
        )
        .await
    }

    pub async fn confirm_payment(
        state: &SessionState,
        request: subscription_types::ConfirmPaymentsRequestData,
        payment_id: String,
        merchant_id: &str,
        profile_id: &str,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        let base_url = &state.conf.internal_services.payments_base_url;
        let url = format!("{}/payments/{}/confirm", base_url, payment_id);

        Self::make_payment_api_call(
            state,
            services::Method::Post,
            url,
            Some(common_utils::request::RequestContent::Json(Box::new(
                request,
            ))),
            "Confirm Payment",
            merchant_id,
            profile_id,
        )
        .await
    }

    pub async fn sync_payment(
        state: &SessionState,
        payment_id: String,
        merchant_id: &str,
        profile_id: &str,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        let base_url = &state.conf.internal_services.payments_base_url;
        let url = format!("{}/payments/{}", base_url, payment_id);

        Self::make_payment_api_call(
            state,
            services::Method::Get,
            url,
            None,
            "Sync Payment",
            merchant_id,
            profile_id,
        )
        .await
    }

    pub async fn create_mit_payment(
        state: &SessionState,
        request: subscription_types::CreateMitPaymentRequestData,
        merchant_id: &str,
        profile_id: &str,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        let base_url = &state.conf.internal_services.payments_base_url;
        let url = format!("{}/payments", base_url);

        Self::make_payment_api_call(
            state,
            services::Method::Post,
            url,
            Some(common_utils::request::RequestContent::Json(Box::new(
                request,
            ))),
            "Create MIT Payment",
            merchant_id,
            profile_id,
        )
        .await
    }

    pub async fn update_payment(
        state: &SessionState,
        request: subscription_types::CreatePaymentsRequestData,
        payment_id: String,
        merchant_id: &str,
        profile_id: &str,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        let base_url = &state.conf.internal_services.payments_base_url;
        let url = format!("{}/payments/{}", base_url, payment_id);

        Self::make_payment_api_call(
            state,
            services::Method::Post,
            url,
            Some(common_utils::request::RequestContent::Json(Box::new(
                request,
            ))),
            "Update Payment",
            merchant_id,
            profile_id,
        )
        .await
    }
}
