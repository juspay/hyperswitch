use masking::Secret;
use router::types::{self, api, storage::enums, AccessToken};

use crate::{
    connector_auth,
    utils::{self, Connector, ConnectorActions},
};

#[derive(Clone, Copy)]
struct IntuitTest;
impl ConnectorActions for IntuitTest {}
impl Connector for IntuitTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Intuit;
        types::api::ConnectorData {
            connector: Box::new(&Intuit),
            connector_name: types::Connector::Intuit,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .intuit
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "intuit".to_string()
    }
}

static CONNECTOR: IntuitTest = IntuitTest {};

fn get_access_token() -> Option<AccessToken> {
    match CONNECTOR.get_auth_token() {
        types::ConnectorAuthType::BodyKey { api_key, key1 } => Some(AccessToken {
            token: api_key,
            expires: key1.parse::<i64>().unwrap(),
        }),
        _ => None,
    }
}
fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        access_token: get_access_token(),
        ..Default::default()
    })
}
// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(None, get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(None, None, get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            None,
            Some(types::PaymentsCaptureData {
                amount_to_capture: Some(50),
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(None, get_default_payment_info())
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
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Payments Void URL is documented incorrectly on gateway's end"]
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            None,
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(None, None, None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[serial_test::serial]
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
#[serial_test::serial]
#[actix_web::test]
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(None, None, None, get_default_payment_info())
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            "".to_string(),
            Some(types::RefundsData {
                amount: 1000,
                currency: enums::Currency::USD,
                refund_id: uuid::Uuid::new_v4().to_string(),
                connector_transaction_id: refund_response.request.connector_transaction_id,
                refund_amount: 100,
                connector_metadata: None,
                reason: None,
                connector_refund_id: Some(refund_response.response.unwrap().connector_refund_id),
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

// Creates a payment using the automatic capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(None, get_default_payment_info())
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
                encoded_data: None,
                capture_method: None,
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(None, None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
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
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    CONNECTOR
        .make_payment_and_multiple_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await;
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(None, None, get_default_payment_info())
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            "".to_string(),
            Some(types::RefundsData {
                amount: 1000,
                currency: enums::Currency::USD,
                refund_id: uuid::Uuid::new_v4().to_string(),
                connector_transaction_id: refund_response.request.connector_transaction_id,
                refund_amount: 100,
                connector_metadata: None,
                reason: None,
                connector_refund_id: Some(refund_response.response.unwrap().connector_refund_id),
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

// Cards Negative scenerios
// Creates a payment with incorrect card number.
#[serial_test::serial]
#[actix_web::test]
async fn should_fail_payment_for_incorrect_card_number() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::Card {
                    card_number: Secret::new("1234567891011".to_string()),
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
        "card.number is invalid.",
    );
}

// Creates a payment with empty card number.
#[serial_test::serial]
#[actix_web::test]
async fn should_fail_payment_for_empty_card_number() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::Card {
                    card_number: Secret::new(String::from("")),
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
        "card.cvc is invalid.",
    );
}

// Creates a payment with incorrect CVC.
#[serial_test::serial]
#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::Card {
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
        "card.cvc is invalid.",
    );
}

// Creates a payment with incorrect expiry month.
#[serial_test::serial]
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::Card {
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
        "card.expmonth is invalid.",
    );
}

// Creates a payment with incorrect expiry year.
#[serial_test::serial]
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::Card {
                    card_exp_year: Secret::new("2000".to_string()),
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
        "card.expyear is invalid.",
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "uri is invalid."
    );
}

// Captures a payment using invalid connector payment id.
#[serial_test::serial]
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();

    assert_eq!(
        capture_response.response.unwrap_err().message,
        "chargeId is invalid."
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[serial_test::serial]
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.response.unwrap_err().message, "amount is invalid.",);
}
