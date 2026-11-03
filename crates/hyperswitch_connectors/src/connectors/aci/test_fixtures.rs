//! Router-data fixtures shared by the ACI unit tests and the golden wire tests.

use common_enums::enums;
use common_utils::{
    id_type,
    types::{MinorUnit, StringMajorUnit},
};
use hyperswitch_domain_models::{
    mandates::{self, MandateReferenceId},
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_request_types::{PaymentsAuthorizeData, PaymentsSyncData},
};
use hyperswitch_masking::Secret;

use super::{
    cof_record::AciCofStash,
    transformers::request::{
        AciBillingAddress, AciCustomParameters, AciCustomerBrowserInfo, AciCustomerData,
        AciExternalThreeDsData, AciPaymentType, AciPaymentsRequest, PaymentDetails,
        TransactionDetails, ACI_CHALLENGE_WINDOW_FULL_SCREEN,
    },
};

pub(crate) const COF_RG: &str = "8ac7a4a19a1b2c3d019a1b2c3d4e0001";
pub(crate) const COF_CITI: &str = "MDGXVJV4N0910";
pub(crate) const COF_TLID: &str = "TLID0001";

pub(crate) fn cof_authorize_data() -> PaymentsAuthorizeData {
    PaymentsAuthorizeData {
        enable_avs_check: None,
        payment_method_data: PaymentMethodData::MandatePayment,
        amount: 1000,
        order_tax_amount: None,
        email: None,
        customer_name: None,
        currency: enums::Currency::ZAR,
        confirm: true,
        capture_method: Some(enums::CaptureMethod::Automatic),
        router_return_url: None,
        webhook_url: None,
        complete_authorize_url: None,
        setup_future_usage: None,
        mandate_id: None,
        off_session: None,
        customer_acceptance: None,
        setup_mandate_details: None,
        browser_info: None,
        order_details: None,
        order_category: None,
        session_token: None,
        enrolled_for_3ds: false,
        related_transaction_id: None,
        payment_experience: None,
        payment_method_type: None,
        surcharge_details: None,
        customer_id: None,
        request_incremental_authorization: false,
        metadata: None,
        authentication_data: None,
        ucs_authentication_data: None,
        force_3ds_challenge: None,
        request_extended_authorization: None,
        split_payments: None,
        guest_customer: None,
        minor_amount: MinorUnit::new(1000),
        merchant_order_reference_id: None,
        integrity_object: None,
        shipping_cost: None,
        additional_payment_method_data: None,
        merchant_account_id: None,
        merchant_config_currency: None,
        connector_testing_data: None,
        order_id: None,
        locale: None,
        payment_channel: None,
        enable_partial_authorization: None,
        enable_overcapture: None,
        is_stored_credential: None,
        mit_category: None,
        billing_descriptor: None,
        tokenization: None,
        partner_merchant_identifier_details: None,
        feature_metadata: None,
        installment_details: None,
        connector_intent_metadata: None,
        is_account_funded_transaction: None,
        recipient_details: None,
        business_country: None,
    }
}

/// A registration-creating CIT: `setup_future_usage=off_session` with customer acceptance.
pub(crate) fn cit_authorize_data() -> PaymentsAuthorizeData {
    PaymentsAuthorizeData {
        setup_future_usage: Some(enums::FutureUsage::OffSession),
        customer_acceptance: Some(common_types::payments::CustomerAcceptance::default()),
        ..cof_authorize_data()
    }
}

pub(crate) fn charged_record(
    rg: &str,
    mandate_metadata: Option<serde_json::Value>,
    request_reference: Option<&str>,
) -> mandates::MandateIds {
    mandates::MandateIds {
        mandate_id: Some("man_test".to_string()),
        mandate_reference_id: Some(MandateReferenceId::ConnectorMandateId(
            mandates::ConnectorMandateReferenceId::new(
                Some(rg.to_string()),
                None,
                None,
                mandate_metadata.map(Secret::new),
                request_reference.map(String::from),
                None,
            ),
        )),
    }
}

/// A MIT charging `rg` with the given mandate record.
pub(crate) fn mit_authorize_data(
    rg: &str,
    mandate_metadata: Option<serde_json::Value>,
    request_reference: Option<&str>,
) -> PaymentsAuthorizeData {
    PaymentsAuthorizeData {
        mandate_id: Some(charged_record(rg, mandate_metadata, request_reference)),
        off_session: Some(true),
        ..cof_authorize_data()
    }
}

pub(crate) fn sync_data(stash: Option<&AciCofStash>) -> PaymentsSyncData {
    PaymentsSyncData {
        connector_meta: stash.map(AciCofStash::to_connector_meta),
        ..Default::default()
    }
}

pub(crate) fn cof_router_data<F, Req, Resp>(request: Req) -> RouterData<F, Req, Resp> {
    RouterData {
        accept_amount_mismatch: None,
        flow: std::marker::PhantomData,
        merchant_id: id_type::MerchantId::default(),
        customer_id: None,
        connector_customer: None,
        connector: "aci".to_string(),
        payment_id: "pay_cof_test".to_string(),
        attempt_id: "pay_cof_test_1".to_string(),
        tenant_id: id_type::TenantId::get_default_global_tenant_id(),
        status: enums::AttemptStatus::Pending,
        payment_method: enums::PaymentMethod::Card,
        payment_method_type: None,
        connector_auth_type: ConnectorAuthType::NoKey,
        description: None,
        address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
        auth_type: enums::AuthenticationType::NoThreeDs,
        connector_meta_data: None,
        connector_wallets_details: None,
        amount_captured: None,
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
        connector_request_reference_id: "pay_cof_test_1".to_string(),
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
        payout_id: None,
        connector_response: None,
        payment_method_status: None,
        minor_amount_captured: None,
        minor_amount_capturable: None,
        authorized_amount: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        l2_l3_data: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        customer_document_details: None,
        customer_date_of_birth: None,
        feature_data: None,
        sender_payment_instrument_id: None,
        connector_returned_payment_method_details: None,
    }
}

// ---- composed request bodies ----

pub(crate) fn customer_request(
    customer_browser_info: AciCustomerBrowserInfo,
    customer_data: AciCustomerData,
    payment_method: PaymentDetails,
    three_ds_two_enrolled: Option<bool>,
) -> AciPaymentsRequest {
    AciPaymentsRequest {
        txn_details: TransactionDetails {
            entity_id: Secret::new("8ac7a4c9".to_string()),
            amount: StringMajorUnit::zero(),
            currency: "ZAR".to_string(),
            payment_type: AciPaymentType::Debit,
        },
        payment_method,
        instruction: None,
        shopper_result_url: None,
        three_ds_two_enrolled,
        recurring_type: None,
        transaction_category: None,
        merchant_transaction_id: None,
        descriptor: None,
        test_mode: None,
        customer_browser_info,
        customer_data,
        billing_address: AciBillingAddress::default(),
        external_three_ds: AciExternalThreeDsData::default(),
        custom_parameters: AciCustomParameters::default(),
    }
}

/// How many times `key` appears as a form key in an encoded body.
pub(crate) fn key_occurrences(body: &str, key: &str) -> usize {
    let prefix = format!("{key}=");
    body.split('&')
        .filter(|pair| pair.starts_with(&prefix))
        .count()
}

pub(crate) fn populated_browser_info() -> AciCustomerBrowserInfo {
    AciCustomerBrowserInfo {
        accept_header: Some("text/html".to_string()),
        language: Some("en-ZA".to_string()),
        screen_height: Some("1080".to_string()),
        screen_width: Some("1920".to_string()),
        timezone: Some("-120".to_string()),
        user_agent: Some("Mozilla/5.0".to_string()),
        javascript_enabled: Some(true),
        java_enabled: Some(false),
        color_depth: Some("24".to_string()),
        challenge_window: Some(ACI_CHALLENGE_WINDOW_FULL_SCREEN.to_string()),
    }
}

pub(crate) fn shopper_customer_data() -> AciCustomerData {
    AciCustomerData {
        merchant_customer_id: Some(Secret::new(
            id_type::CustomerId::wrap("cus_shopper_1".to_string()).expect("valid customer id"),
        )),
        ..Default::default()
    }
}
