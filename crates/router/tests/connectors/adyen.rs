use api_models::payments::{Address, AddressDetails};
use masking::Secret;
use router::types::{self, api, storage::enums, PaymentAddress};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentInfo},
};

#[derive(Clone, Copy)]
struct AdyenTest;
impl ConnectorActions for AdyenTest {}
impl utils::Connector for AdyenTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Adyen;
        types::api::ConnectorData {
            connector: Box::new(&Adyen),
            connector_name: types::Connector::Adyen,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .adyen
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "adyen".to_string()
    }
}

impl AdyenTest {
    fn get_payment_info() -> Option<PaymentInfo> {
        Some(PaymentInfo {
            address: Some(PaymentAddress {
                billing: Some(Address {
                    address: Some(AddressDetails {
                        country: Some("US".to_string()),
                        ..Default::default()
                    }),
                    phone: None,
                }),
                ..Default::default()
            }),
            router_return_url: Some(String::from("http://localhost:8080")),
            ..Default::default()
        })
    }

    fn get_payment_authorize_data(
        card_number: &str,
        card_exp_month: &str,
        card_exp_year: &str,
        card_cvc: &str,
        capture_method: enums::CaptureMethod,
    ) -> Option<types::PaymentsAuthorizeData> {
        Some(types::PaymentsAuthorizeData {
            amount: 3500,
            currency: enums::Currency::USD,
            payment_method_data: types::api::PaymentMethodData::Card(types::api::Card {
                card_number: Secret::new(card_number.to_string()),
                card_exp_month: Secret::new(card_exp_month.to_string()),
                card_exp_year: Secret::new(card_exp_year.to_string()),
                card_holder_name: Secret::new("John Doe".to_string()),
                card_cvc: Secret::new(card_cvc.to_string()),
                card_issuer: None,
                card_network: None,
            }),
            confirm: true,
            statement_descriptor_suffix: None,
            statement_descriptor: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            capture_method: Some(capture_method),
            browser_info: None,
            order_details: None,
            email: None,
            payment_experience: None,
            payment_method_type: None,
            session_token: None,
            enrolled_for_3ds: false,
            related_transaction_id: None,
        })
    }
}

static CONNECTOR: AdyenTest = AdyenTest {};

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(
            AdyenTest::get_payment_authorize_data(
                "4111111111111111",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Manual,
            ),
            AdyenTest::get_payment_info(),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            AdyenTest::get_payment_authorize_data(
                "370000000000002",
                "03",
                "2030",
                "7373",
                enums::CaptureMethod::Manual,
            ),
            None,
            AdyenTest::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            AdyenTest::get_payment_authorize_data(
                "4293189100000008",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Manual,
            ),
            Some(types::PaymentsCaptureData {
                amount_to_capture: Some(50),
                ..utils::PaymentCaptureType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            AdyenTest::get_payment_authorize_data(
                "4293189100000008",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Manual,
            ),
            Some(types::PaymentsCancelData {
                amount: None,
                currency: None,
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            AdyenTest::get_payment_authorize_data(
                "370000000000002",
                "03",
                "2030",
                "7373",
                enums::CaptureMethod::Manual,
            ),
            None,
            Some(types::RefundsData {
                refund_amount: 1500,
                reason: Some("CUSTOMER REQUEST".to_string()),
                ..utils::PaymentRefundType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            AdyenTest::get_payment_authorize_data(
                "2222400070000005",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Manual,
            ),
            None,
            Some(types::RefundsData {
                refund_amount: 1500,
                reason: Some("CUSTOMER REQUEST".to_string()),
                ..utils::PaymentRefundType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            AdyenTest::get_payment_authorize_data(
                "2222400070000005",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Manual,
            ),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(
            AdyenTest::get_payment_authorize_data(
                "2222400070000005",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Automatic,
            ),
            Some(types::RefundsData {
                refund_amount: 1000,
                reason: Some("CUSTOMER REQUEST".to_string()),
                ..utils::PaymentRefundType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            AdyenTest::get_payment_authorize_data(
                "4293189100000008",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Automatic,
            ),
            Some(types::RefundsData {
                refund_amount: 500,
                reason: Some("CUSTOMER REQUEST".to_string()),
                ..utils::PaymentRefundType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    CONNECTOR
        .make_payment_and_multiple_refund(
            AdyenTest::get_payment_authorize_data(
                "2222400070000005",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Automatic,
            ),
            Some(types::RefundsData {
                refund_amount: 100,
                reason: Some("CUSTOMER REQUEST".to_string()),
                ..utils::PaymentRefundType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await;
}

// Cards Negative scenerios
// Creates a payment with incorrect card number.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_card_number() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: Secret::new("1234567891011".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Invalid card number",
    );
}

// Creates a payment with empty card number.
#[actix_web::test]
async fn should_fail_payment_for_empty_card_number() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: Secret::new(String::from("")),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "Missing payment method details: number",);
}

// Creates a payment with incorrect CVC.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_cvc: Secret::new("12345".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "CVC is not the right length",
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_month: Secret::new("20".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "The provided Expiry Date is not valid.: Expiry month should be between 1 and 12 inclusive: 20",
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_year: Secret::new("2000".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.response.unwrap_err().message, "Expired Card",);
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, AdyenTest::get_payment_info())
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("Original pspReference required for this operation")
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
