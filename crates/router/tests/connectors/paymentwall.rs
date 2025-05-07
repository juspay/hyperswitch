use std::str::FromStr;

use api_models::payments::{Address, AddressDetails, CardNetworkTypes};
use hyperswitch_domain_models::router_data::PaymentMethodData;
use hyperswitch_interfaces::types::PaymentsAuthorizeType;
use masking::Secret;
use router::types::{self, api, domain, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentInfo},
};

#[test]
#[ignore]
fn should_only_authorize_payment() {
    let connector = utils::get_connector("paymentwall");
    let payment_info = PaymentInfo {
        payment_method_data: types::PaymentMethodData::Card(api::Card {
            card_number: Secret::new("4242424242424242".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_cvc: Secret::new("123".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(Secret::new("nick_name".to_string())),
        }),
        currency: enums::Currency::USD,
        capture_method: Some(enums::CaptureMethod::Manual),
        ..utils::PaymentInfo::default()
    };
    let expected_status = vec![enums::AttemptStatus::Authorized];
    connector_auth::make_payment_connector_request(
        &connector,
        PaymentsAuthorizeType::Authorize,
        payment_info,
        expected_status,
    );
}

#[test]
#[ignore]
fn should_authorize_and_capture_payment() {
    let connector = utils::get_connector("paymentwall");
    let payment_info = PaymentInfo {
        payment_method_data: types::PaymentMethodData::Card(api::Card {
            card_number: Secret::new("4242424242424242".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_cvc: Secret::new("123".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(Secret::new("nick_name".to_string())),
        }),
        currency: enums::Currency::USD,
        capture_method: None,
        ..utils::PaymentInfo::default()
    };
    let expected_status = vec![enums::AttemptStatus::Charged];
    connector_auth::make_payment_connector_request(
        &connector,
        PaymentsAuthorizeType::Authorize,
        payment_info,
        expected_status,
    );
}

#[test]
#[ignore]
fn should_authorize_and_capture_with_manual_capture() {
    let connector = utils::get_connector("paymentwall");
    let payment_info = PaymentInfo {
        payment_method_data: types::PaymentMethodData::Card(api::Card {
            card_number: Secret::new("4242424242424242".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_cvc: Secret::new("123".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(Secret::new("nick_name".to_string())),
        }),
        currency: enums::Currency::USD,
        capture_method: Some(enums::CaptureMethod::Manual),
        ..utils::PaymentInfo::default()
    };
    let connector_payment_id = connector_auth::make_payment_connector_request(
        &connector,
        PaymentsAuthorizeType::Authorize,
        payment_info.clone(),
        vec![enums::AttemptStatus::Authorized],
    );
    let capture_payment_info = PaymentInfo {
        payment_id: connector_payment_id,
        ..payment_info
    };
    connector_auth::make_payment_connector_request(
        &connector,
        PaymentsAuthorizeType::Capture,
        capture_payment_info,
        vec![enums::AttemptStatus::Charged],
    );
}

#[test]
#[ignore]
fn should_authorize_and_void_payment() {
    let connector = utils::get_connector("paymentwall");
    let payment_info = PaymentInfo {
        payment_method_data: types::PaymentMethodData::Card(api::Card {
            card_number: Secret::new("4242424242424242".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_cvc: Secret::new("123".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(Secret::new("nick_name".to_string())),
        }),
        currency: enums::Currency::USD,
        capture_method: Some(enums::CaptureMethod::Manual),
        ..utils::PaymentInfo::default()
    };
    let connector_payment_id = connector_auth::make_payment_connector_request(
        &connector,
        PaymentsAuthorizeType::Authorize,
        payment_info.clone(),
        vec![enums::AttemptStatus::Authorized],
    );
    let void_payment_info = PaymentInfo {
        payment_id: connector_payment_id,
        ..payment_info
    };
    connector_auth::make_payment_connector_request(
        &connector,
        PaymentsAuthorizeType::Void,
        void_payment_info,
        vec![enums::AttemptStatus::Voided],
    );
}

#[test]
#[ignore]
fn should_authorize_sync_payment() {
    let connector = utils::get_connector("paymentwall");
    let payment_info = PaymentInfo {
        payment_method_data: types::PaymentMethodData::Card(api::Card {
            card_number: Secret::new("4242424242424242".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_cvc: Secret::new("123".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(Secret::new("nick_name".to_string())),
        }),
        currency: enums::Currency::USD,
        ..utils::PaymentInfo::default()
    };
    let connector_payment_id = connector_auth::make_payment_connector_request(
        &connector,
        PaymentsAuthorizeType::Authorize,
        payment_info.clone(),
        vec![enums::AttemptStatus::Charged],
    );
    let sync_payment_info = PaymentInfo {
        payment_id: connector_payment_id,
        ..payment_info
    };
    connector_auth::make_payment_connector_request(
        &connector,
        PaymentsAuthorizeType::PSync,
        sync_payment_info,
        vec![enums::AttemptStatus::Charged],
    );
}

#[test]
#[ignore]
fn should_make_payment_and_refund() {
    let connector = utils::get_connector("paymentwall");
    let payment_info = PaymentInfo {
        payment_method_data: types::PaymentMethodData::Card(api::Card {
            card_number: Secret::new("4242424242424242".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_cvc: Secret::new("123".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(Secret::new("nick_name".to_string())),
        }),
        currency: enums::Currency::USD,
        ..utils::PaymentInfo::default()
    };
    let connector_payment_id = connector_auth::make_payment_connector_request(
        &connector,
        PaymentsAuthorizeType::Authorize,
        payment_info.clone(),
        vec![enums::AttemptStatus::Charged],
    );
    let refund_info = PaymentInfo {
        payment_id: connector_payment_id,
        refund_amount: 100,
        ..payment_info
    };
    connector_auth::make_refund_connector_request(
        &connector,
        refund_info,
        vec![enums::RefundStatus::Success],
    );
}

#[test]
#[ignore]
fn should_refund_sync() {
    let connector = utils::get_connector("paymentwall");
    let payment_info = PaymentInfo {
        payment_method_data: types::PaymentMethodData::Card(api::Card {
            card_number: Secret::new("4242424242424242".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_cvc: Secret::new("123".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(Secret::new("nick_name".to_string())),
        }),
        currency: enums::Currency::USD,
        ..utils::PaymentInfo::default()
    };
    let connector_payment_id = connector_auth::make_payment_connector_request(
        &connector,
        PaymentsAuthorizeType::Authorize,
        payment_info.clone(),
        vec![enums::AttemptStatus::Charged],
    );
    let refund_info = PaymentInfo {
        payment_id: connector_payment_id,
        refund_amount: 100,
        ..payment_info
    };
    let connector_refund_id = connector_auth::make_refund_connector_request(
        &connector,
        refund_info.clone(),
        vec![enums::RefundStatus::Success],
    );
    let sync_refund_info = PaymentInfo {
        payment_id: connector_payment_id,
        refund_id: connector_refund_id,
        ..refund_info
    };
    connector_auth::make_refund_sync_connector_request(
        &connector,
        sync_refund_info,
        vec![enums::RefundStatus::Success],
    );
}

#[test]
#[ignore]
fn should_setup_mandate_and_use_for_payment() {
    let connector = utils::get_connector("paymentwall");
    let payment_info = PaymentInfo {
        payment_method_data: types::PaymentMethodData::Card(api::Card {
            card_number: Secret::new("4242424242424242".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_cvc: Secret::new("123".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(Secret::new("nick_name".to_string())),
        }),
        currency: enums::Currency::USD,
        setup_mandate_details: Some(api_models::payments::MandateData {
            mandate_type: api_models::payments::MandateType::SingleUse,
            ..Default::default()
        }),
        ..utils::PaymentInfo::default()
    };
    let connector_mandate_id = connector_auth::make_payment_connector_request(
        &connector,
        PaymentsAuthorizeType::SetupMandate,
        payment_info.clone(),
        vec![enums::AttemptStatus::Charged],
    );
    let mandate_payment_info = PaymentInfo {
        payment_method_data: types::PaymentMethodData::MandatePayment {
            mandate_id: connector_mandate_id,
            customer_acceptance: None,
        },
        currency: enums::Currency::USD,
        ..utils::PaymentInfo::default()
    };
    connector_auth::make_payment_connector_request(
        &connector,
        PaymentsAuthorizeType::Authorize,
        mandate_payment_info,
        vec![enums::AttemptStatus::Charged],
    );
}