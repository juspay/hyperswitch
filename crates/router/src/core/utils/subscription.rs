use common_utils::ext_traits::OptionExt;
use diesel_models::subscription::Subscription;
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data::{ErrorResponse, RouterData};

use crate::{consts, db::errors, routes::SessionState};

pub fn authenticate_subscription_client_secret_and_check_expiry(
    req_client_secret: &String,
    subscription: &Subscription,
) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
    let stored_client_secret = subscription
        .client_secret
        .clone()
        .get_required_value("client_secret")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "client_secret",
        })
        .attach_printable("client secret not found in db")?;

    if req_client_secret != &stored_client_secret {
        Err((errors::ApiErrorResponse::ClientSecretInvalid).into())
    } else {
        let current_timestamp = common_utils::date_time::now();
        let session_expiry = subscription
            .created_at
            .saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY));

        let expired = current_timestamp > session_expiry;

        Ok(expired)
    }
}

pub fn create_subscription_router_data<F, Req, Res>(
    state: &SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    customer_id: Option<common_utils::id_type::CustomerId>,
    connector_name: String,
    auth_type: hyperswitch_domain_models::router_data::ConnectorAuthType,
    request: Req,
    payment_id: common_utils::id_type::PaymentId,
) -> common_utils::errors::CustomResult<RouterData<F, Req, Res>, errors::ApiErrorResponse>
where
    F: Clone,
{
    let site: masking::Secret<String> =
        masking::Secret::from("hyperswitch-juspay2-test".to_string());
    let test_data = common_utils::pii::SecretSerdeValue::new(serde_json::json!({
        "site": site,
    }));
    Ok(RouterData {
        flow: std::marker::PhantomData,
        merchant_id,
        customer_id,
        connector_customer: None,
        connector: connector_name,
        payment_id: payment_id.get_string_repr().to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        attempt_id: "Subscriptions attempt".to_owned(),
        status: common_enums::AttemptStatus::default(),
        payment_method: common_enums::PaymentMethod::default(),
        connector_auth_type: auth_type,
        description: None,
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: Some(test_data),
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
        request,
        response: Err(ErrorResponse::default()),
        connector_request_reference_id: "Nothing".to_owned(),
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
