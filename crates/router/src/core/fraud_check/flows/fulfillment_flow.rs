use common_utils::ext_traits::{OptionExt, ValueExt};
use error_stack::ResultExt;
use router_env::tracing::{self, instrument};

use crate::{
    core::{
        errors::RouterResult, fraud_check::frm_core_types::FrmFulfillmentRequest,
        payments::helpers, utils as core_utils,
    },
    errors,
    types::{
        domain,
        fraud_check::{FraudCheckFulfillmentData, FrmFulfillmentRouterData},
        storage, ConnectorAuthType, ErrorResponse, PaymentAddress, RouterData,
    },
    utils, SessionState,
};

#[cfg(feature = "v2")]
pub async fn construct_fulfillment_router_data<'a>(
    _state: &'a SessionState,
    _payment_intent: &'a storage::PaymentIntent,
    _payment_attempt: &storage::PaymentAttempt,
    _platform: &domain::Platform,
    _connector: String,
    _fulfillment_request: FrmFulfillmentRequest,
) -> RouterResult<FrmFulfillmentRouterData> {
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn construct_fulfillment_router_data<'a>(
    state: &'a SessionState,
    payment_intent: &'a storage::PaymentIntent,
    payment_attempt: &storage::PaymentAttempt,
    platform: &domain::Platform,
    connector: String,
    fulfillment_request: FrmFulfillmentRequest,
) -> RouterResult<FrmFulfillmentRouterData> {
    let profile_id = payment_intent
        .profile_id
        .as_ref()
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("profile_id is not set in payment_intent")?
        .clone();

    let connector_id = connector.clone();

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        platform.get_processor().get_account().get_id(),
        None,
        platform.get_processor().get_key_store(),
        &profile_id,
        &connector,
        None,
    )
    .await?;

    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let auth_type: ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let payment_method =
        utils::OptionExt::get_required_value(payment_attempt.payment_method, "payment_method")?;

    let router_data = RouterData {
        flow: std::marker::PhantomData,
        merchant_id: platform.get_processor().get_account().get_id().clone(),
        tenant_id: state.tenant.tenant_id.clone(),
        connector,
        payment_id: payment_attempt.payment_id.get_string_repr().to_owned(),
        attempt_id: payment_attempt.attempt_id.clone(),
        status: payment_attempt.status,
        payment_method,
        payment_method_type: payment_attempt.payment_method_type,
        connector_auth_type: auth_type,
        description: None,
        address: PaymentAddress::default(),
        auth_type: payment_attempt.authentication_type.unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: payment_intent
            .amount_captured
            .map(|amt| amt.get_amount_as_i64()),
        minor_amount_captured: payment_intent.amount_captured,
        payment_method_status: None,
        request: FraudCheckFulfillmentData {
            amount: payment_attempt
                .net_amount
                .get_total_amount()
                .get_amount_as_i64(),
            order_details: payment_intent.order_details.clone(),
            fulfillment_req: fulfillment_request,
        },
        response: Err(ErrorResponse::default()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        customer_id: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_request_reference_id: core_utils::get_connector_request_reference_id(
            &state.conf,
            platform.get_processor().get_account().get_id(),
            payment_intent,
            payment_attempt,
            &connector_id,
        )?,
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        connector_api_version: None,
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
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
    };
    Ok(router_data)
}
