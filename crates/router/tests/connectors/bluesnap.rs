use std::str::FromStr;

use api_models::payments::{Address, AddressDetails};
use common_utils::pii::Email;
use masking::Secret;
use router::types::{self, api, storage::enums, ConnectorAuthType, PaymentAddress};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentInfo},
};

#[derive(Clone, Copy)]
struct BluesnapTest;
impl ConnectorActions for BluesnapTest {}
static CONNECTOR: BluesnapTest = BluesnapTest {};
impl utils::Connector for BluesnapTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Bluesnap;
        types::api::ConnectorData {
            connector: Box::new(&Bluesnap),
            connector_name: types::Connector::Bluesnap,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .bluesnap
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "bluesnap".to_string()
    }
}
fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        email: Some(Email::from_str("test@gmail.com").unwrap()),
        ..utils::PaymentAuthorizeType::default().0
    })
}
fn get_payment_info() -> Option<PaymentInfo> {
    Some(PaymentInfo {
        address: Some(PaymentAddress {
            billing: Some(Address {
                address: Some(AddressDetails {
                    first_name: Some(Secret::new("joseph".to_string())),
                    last_name: Some(Secret::new("Doe".to_string())),
                    ..Default::default()
                }),
                phone: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    })
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).

#[serial_test::serial]
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).

#[serial_test::serial]
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(payment_method_details(), None, get_payment_info())
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
            payment_method_details(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            get_payment_info(),
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
        .authorize_payment(payment_method_details(), get_payment_info())
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
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).

#[serial_test::serial]
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
            get_payment_info(),
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
        .capture_payment_and_refund(payment_method_details(), None, None, get_payment_info())
        .await
        .unwrap();
    let rsync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        rsync_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).

#[serial_test::serial]
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
            get_payment_info(),
        )
        .await
        .unwrap();
    let rsync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        rsync_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Synchronizes a refund using the manual capture flow (Non 3DS).

#[serial_test::serial]
#[actix_web::test]
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(payment_method_details(), None, None, get_payment_info())
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

#[serial_test::serial]
#[actix_web::test]
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).

#[serial_test::serial]
#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_payment_info())
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

#[serial_test::serial]
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(payment_method_details(), None, get_payment_info())
        .await
        .unwrap();
    let rsync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        rsync_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).

#[serial_test::serial]
#[actix_web::test]
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_payment_info(),
        )
        .await
        .unwrap();
    let rsync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        rsync_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).

#[serial_test::serial]
#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let transaction_id = utils::get_connector_transaction_id(authorize_response.response).unwrap();
    for _x in 0..2 {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await; // to avoid 404 error
        let refund_response = CONNECTOR
            .refund_payment(
                transaction_id.clone(),
                Some(types::RefundsData {
                    refund_amount: 50,
                    ..utils::PaymentRefundType::default().0
                }),
                None,
            )
            .await
            .unwrap();
        let rsync_response = CONNECTOR
            .rsync_retry_till_status_matches(
                enums::RefundStatus::Success,
                refund_response.response.unwrap().connector_refund_id,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            rsync_response.response.unwrap().refund_status,
            enums::RefundStatus::Success,
        );
    }
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).

#[serial_test::serial]
#[actix_web::test]
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(payment_method_details(), None, get_payment_info())
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

// Creates a payment with incorrect CVC.

#[serial_test::serial]
#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                email: Some(Email::from_str("test@gmail.com").unwrap()),
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_holder_name: Secret::new("John Doe".to_string()),
                    card_cvc: Secret::new("12345".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "VALIDATION_GENERAL_FAILURE".to_string(),
    );
}

// Creates a payment with incorrect expiry month.

#[serial_test::serial]
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                email: Some(Email::from_str("test@gmail.com").unwrap()),
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_holder_name: Secret::new("John Doe".to_string()),
                    card_exp_month: Secret::new("20".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "VALIDATION_GENERAL_FAILURE".to_string(),
    );
}

// Creates a payment with incorrect expiry year.

#[serial_test::serial]
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                email: Some(Email::from_str("test@gmail.com").unwrap()),
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_holder_name: Secret::new("John Doe".to_string()),
                    card_exp_year: Secret::new("2000".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "VALIDATION_GENERAL_FAILURE".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).

#[serial_test::serial]
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, None)
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "TRANSACTION_ALREADY_CAPTURED"
    );
}

// Captures a payment using invalid connector payment id.

#[serial_test::serial]
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, None)
        .await
        .unwrap();

    capture_response
        .response
        .unwrap_err()
        .message
        .contains("is not authorized to view transaction");
}

// Refunds a payment with refund amount higher than payment amount.

#[serial_test::serial]
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "REFUND_MAX_AMOUNT_FAILURE",
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
