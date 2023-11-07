use std::str::FromStr;

use api_models::payments::OrderDetailsWithAmount;
use cards::CardNumber;
use common_utils::pii::Email;
use masking::Secret;
use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct ZenTest;
impl ConnectorActions for ZenTest {}
impl utils::Connector for ZenTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Zen;
        types::api::ConnectorData {
            connector: Box::new(&Zen),
            connector_name: types::Connector::Zen,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .zen
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "zen".to_string()
    }
}

static CONNECTOR: ZenTest = ZenTest {};

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card"]
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(None, None)
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card and capture is not supported"]
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(None, None, None)
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card and capture is not supported"]
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            None,
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card"]
#[actix_web::test]
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(None, None)
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                encoded_data: None,
                capture_method: None,
                sync_type: types::SyncRequestType::SinglePaymentSync,
                connector_meta: None,
                mandate_id: None,
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card and void is not supported"]
#[actix_web::test]
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            None,
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card and capture is not supported"]
#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(None, None, None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card and capture is not supported"]
#[actix_web::test]
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            None,
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Synchronizes a refund using the manual capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card and capture is not supported"]
#[actix_web::test]
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(None, None, None, None)
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

// Creates a payment using the automatic capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card"]
#[actix_web::test]
async fn should_make_payment() {
    let authorize_response = CONNECTOR.make_payment(None, None).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card"]
#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR.make_payment(None, None).await.unwrap();
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
                encoded_data: None,
                capture_method: Some(enums::CaptureMethod::Automatic),
                sync_type: types::SyncRequestType::SinglePaymentSync,
                connector_meta: None,
                mandate_id: None,
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card"]
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(None, None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card"]
#[actix_web::test]
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card"]
#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    CONNECTOR
        .make_payment_and_multiple_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await;
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card"]
#[actix_web::test]
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(None, None, None)
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
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: CardNumber::from_str("1234567891011").unwrap(),
                    ..utils::CCardType::default().0
                }),
                order_details: Some(vec![OrderDetailsWithAmount {
                    product_name: "test".to_string(),
                    quantity: 1,
                    amount: 1000,
                    product_img_link: None,
                }]),
                email: Some(Email::from_str("test@gmail.com").unwrap()),
                webhook_url: Some("https://1635-116-74-253-164.ngrok-free.app".to_string()),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response
            .response
            .unwrap_err()
            .message
            .split_once(';')
            .unwrap()
            .0,
        "Request data doesn't pass validation".to_string(),
    );
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
                order_details: Some(vec![OrderDetailsWithAmount {
                    product_name: "test".to_string(),
                    quantity: 1,
                    amount: 1000,
                    product_img_link: None,
                }]),
                email: Some(Email::from_str("test@gmail.com").unwrap()),
                webhook_url: Some("https://1635-116-74-253-164.ngrok-free.app".to_string()),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response
            .response
            .unwrap_err()
            .message
            .split_once(';')
            .unwrap()
            .0,
        "Request data doesn't pass validation".to_string(),
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
                order_details: Some(vec![OrderDetailsWithAmount {
                    product_name: "test".to_string(),
                    quantity: 1,
                    amount: 1000,
                    product_img_link: None,
                }]),
                email: Some(Email::from_str("test@gmail.com").unwrap()),
                webhook_url: Some("https://1635-116-74-253-164.ngrok-free.app".to_string()),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response
            .response
            .unwrap_err()
            .message
            .split_once(';')
            .unwrap()
            .0,
        "Request data doesn't pass validation".to_string(),
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
                order_details: Some(vec![OrderDetailsWithAmount {
                    product_name: "test".to_string(),
                    quantity: 1,
                    amount: 1000,
                    product_img_link: None,
                }]),
                email: Some(Email::from_str("test@gmail.com").unwrap()),
                webhook_url: Some("https://1635-116-74-253-164.ngrok-free.app".to_string()),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response
            .response
            .unwrap_err()
            .message
            .split_once(';')
            .unwrap()
            .0,
        "Request data doesn't pass validation".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[ignore = "Connector triggers 3DS payment on test card and void is not supported"]
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR.make_payment(None, None).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, None)
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "You cannot cancel this PaymentIntent because it has a status of succeeded."
    );
}

// Captures a payment using invalid connector payment id.
#[ignore = "Connector triggers 3DS payment on test card and capture is not supported"]
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("No such payment_intent: '123456789'")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[ignore = "Connector triggers 3DS payment on test card"]
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Refund amount (₹1.50) is greater than charge amount (₹1.00)",
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
