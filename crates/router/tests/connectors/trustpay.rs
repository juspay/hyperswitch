use masking::Secret;
use router::types::{self, api, storage::enums, BrowserInformation};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct TrustpayTest;
impl ConnectorActions for TrustpayTest {}
impl utils::Connector for TrustpayTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Trustpay;
        types::api::ConnectorData {
            connector: Box::new(&Trustpay),
            connector_name: types::Connector::Trustpay,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> common_enums::ConnectorAuthType {
        common_enums::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .trustpay
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "trustpay".to_string()
    }
}

fn get_default_browser_info() -> BrowserInformation {
    BrowserInformation {
        color_depth: 24,
        java_enabled: false,
        java_script_enabled: true,
        language: "en-US".to_string(),
        screen_height: 1080,
        screen_width: 1920,
        time_zone: 3600,
        accept_header: "*".to_string(),
        user_agent: "none".to_string(),
        ip_address: None,
    }
}

fn get_default_payment_authorize_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: Secret::new("4200000000000000".to_string()),
            card_exp_year: Secret::new("25".to_string()),
            card_cvc: Secret::new("123".to_string()),
            ..utils::CCardType::default().0
        }),
        browser_info: Some(get_default_browser_info()),
        router_return_url: Some(String::from("http://localhost:8080")),
        ..utils::PaymentAuthorizeType::default().0
    })
}

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        address: Some(types::PaymentAddress {
            billing: Some(api::Address {
                address: Some(api::AddressDetails {
                    first_name: Some(Secret::new("first".to_string())),
                    last_name: Some(Secret::new("last".to_string())),
                    line1: Some(Secret::new("line1".to_string())),
                    line2: Some(Secret::new("line2".to_string())),
                    city: Some("city".to_string()),
                    zip: Some(Secret::new("zip".to_string())),
                    country: Some(api_models::enums::CountryCode::IN),
                    ..Default::default()
                }),
                phone: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    })
}

static CONNECTOR: TrustpayTest = TrustpayTest {};

// Cards Positive Tests
// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            get_default_payment_authorize_data(),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(
            get_default_payment_authorize_data(),
            None,
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            get_default_payment_authorize_data(),
            None,
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Cards Negative scenerios
// Creates a payment with incorrect card number.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_card_number() {
    let payment_authorize_data = types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: Secret::new("1234567891011".to_string()),
            card_exp_year: Secret::new("25".to_string()),
            card_cvc: Secret::new("123".to_string()),
            ..utils::CCardType::default().0
        }),
        browser_info: Some(get_default_browser_info()),
        ..utils::PaymentAuthorizeType::default().0
    };
    let response = CONNECTOR
        .make_payment(Some(payment_authorize_data), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Errors { code: 61, description: \"invalid payment data (country or brand)\" }".to_string(),
    );
}

// Creates a payment with empty card number.
#[actix_web::test]
async fn should_fail_payment_for_empty_card_number() {
    let payment_authorize_data = types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: Secret::new("".to_string()),
            card_exp_year: Secret::new("25".to_string()),
            card_cvc: Secret::new("123".to_string()),
            ..utils::CCardType::default().0
        }),
        browser_info: Some(get_default_browser_info()),
        ..utils::PaymentAuthorizeType::default().0
    };
    let response = CONNECTOR
        .make_payment(Some(payment_authorize_data), get_default_payment_info())
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(
        x.message,
        "Errors { code: 61, description: \"invalid payment data (country or brand)\" }",
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let payment_authorize_data = Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: Secret::new("4200000000000000".to_string()),
            card_exp_year: Secret::new("22".to_string()),
            card_cvc: Secret::new("123".to_string()),
            ..utils::CCardType::default().0
        }),
        browser_info: Some(get_default_browser_info()),
        ..utils::PaymentAuthorizeType::default().0
    });
    let response = CONNECTOR
        .make_payment(payment_authorize_data, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Errors { code: 15, description: \"the provided expiration year is not valid\" }"
            .to_string(),
    );
}
