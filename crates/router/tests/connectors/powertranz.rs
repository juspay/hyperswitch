use masking::Secret;
use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct PowertranzTest;
impl ConnectorActions for PowertranzTest {}
impl utils::Connector for PowertranzTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Powertranz;
        types::api::ConnectorData {
            connector: Box::new(&Powertranz),
            connector_name: types::Connector::DummyConnector1,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .powertranz
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "powertranz".to_string()
    }
}

static CONNECTOR: PowertranzTest = PowertranzTest {};

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    None
}

fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    None
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(payment_method_details(), None, get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not support partial capture"]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            payment_method_details(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not support payment sync"]
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
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
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            payment_method_details(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            get_default_payment_info(),
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
            payment_method_details(),
            None,
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

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Synchronizes a refund using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not support payment sync"]
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
            None,
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
            get_default_payment_info(),
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
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not support payment sync"]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
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
                capture_method: Some(enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(payment_method_details(), None, get_default_payment_info())
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
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
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
#[ignore = "Connector does not support multiple refund"]
async fn should_refund_succeeded_payment_multiple_times() {
    CONNECTOR
        .make_payment_and_multiple_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await;
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not support refund sync"]
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(payment_method_details(), None, get_default_payment_info())
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
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

// Cards Negative scenerios
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
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Field is invalid: CardCvv".to_string(),
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
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Field is invalid: CardExpiration".to_string(),
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
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Failure,);
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(void_response.status, enums::AttemptStatus::VoidFailed);
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("Original auth not found")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.response.unwrap_err().message, "Invalid amount",);
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
