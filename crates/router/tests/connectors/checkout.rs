use masking::Secret;
use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};
#[derive(Clone, Copy)]
struct CheckoutTest;
impl ConnectorActions for CheckoutTest {}
impl utils::Connector for CheckoutTest {
        /// This method returns the connector data for the Checkout connector.
    fn get_data(&self) -> types::api::ConnectorData {
            use router::connector::Checkout;
            types::api::ConnectorData {
                connector: Box::new(&Checkout),
                connector_name: types::Connector::Checkout,
                get_token: types::api::GetToken::Connector,
                merchant_connector_id: None,
            }
        }

        /// Retrieves the authentication token for the connector.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .checkout
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// This method returns the name "checkout".
    fn get_name(&self) -> String {
        "checkout".to_string()
    }
}

static CONNECTOR: CheckoutTest = CheckoutTest {};

/// Retrieves the default payment information for the user, if available.
///
/// This method returns an `Option` containing the default payment information if it exists, otherwise it returns `None`.
/// 
fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    None
}
/// Retrieves the details of the payment method used for authorization.
///
/// This method returns an Option containing the payment method details if available, otherwise it returns None.
///
fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    None
}


// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously authorizes a payment and ensures that the payment is only authorized, not captured.
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously attempts to capture an authorized payment using the payment details provided. 
///
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(payment_method_details(), None, get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}
// Partially captures a payment using the manual capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously initiates an authorization and capture payment process with the CONNECTOR, 
/// capturing a specified amount from the payment method. It then validates the response 
/// and asserts that the status is "Charged".
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
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously performs the necessary steps to synchronize an authorized payment. This includes authorizing the payment, retrieving the transaction ID, and then retrying the synchronization process until the status matches the Authorized status. Finally, it asserts that the response status is Authorized.
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
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously authorizes and voids a payment using the connector. It first authorizes the payment with the provided payment method details, then immediately voids the authorized payment with the specified cancellation data and default payment information. It expects a void payment response and asserts that the response status is voided.
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
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously captures a payment and initiates a refund process for the captured payment. 
/// If the refund is successful, it asserts that the refund status is set to `Success`.
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
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously captures a payment and partially refunds it manually.
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
#[ignore = "Connector Error, needs to be looked into and fixed"]
/// Asynchronously captures a payment and initiates a refund, then synchronously retries until the refund status matches the expected success status.
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
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously makes a payment using the CONNECTOR, with the provided payment method details and default payment information.
///
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}
// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
/// This method is an asynchronous function that checks whether auto-captured payment should be synchronized. It first makes a payment using the `CONNECTOR` and then asserts that the status of the authorize response is `Charged`. It then retrieves the transaction ID from the authorize response and asserts that it is not empty. Finally, it performs a retry till the status matches `Charged` with specific payment sync data and asserts that the response status is `Charged`.
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
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously makes a payment, refunds the payment using the default payment information, and asserts that the refund status is 'Success'.
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
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously attempts to partially refund a succeeded payment using the specified payment method details, refund amount, and payment information.
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
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously makes a payment and then requests multiple refunds for the payment. 
/// The method first makes a payment using the specified payment method details and default payment information. 
/// It then proceeds to request multiple refunds for the payment, with each refund amount set to 50. 
/// The method returns once all refund requests have been processed.
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
#[ignore = "Connector Error, needs to be looked into and fixed"]
/// Asynchronously performs a refund operation and checks if the refund is successful by retrying until the status matches. 
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(payment_method_details(), None, get_default_payment_info())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
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
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously makes a payment with an incorrect CVC and asserts that the payment fails with a "cvv_invalid" error message.
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
        "cvv_invalid".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously tests that the payment fails for an invalid expiration month on a credit card.
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
        "card_expiry_month_invalid".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously checks if a payment fails for an incorrect expiry year of the payment card.
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
    assert_eq!(
        response.response.unwrap_err().message,
        "card_expired".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously attempts to make a void payment for an auto-capture scenario, expecting the payment to fail. 
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
    assert_eq!(void_response.response.unwrap_err().status_code, 403);
}

// Captures a payment using invalid connector payment id.
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously attempts to capture a payment with an invalid payment ID and expects the operation to fail with a 404 error status code.
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(capture_response.response.unwrap_err().status_code, 404);
}

// Refunds a payment with refund amount higher than payment amount.
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously performs a payment and refund operation, and asserts that the response
/// contains an error message indicating that the refund amount exceeds the balance.
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
    assert_eq!(
        response.response.unwrap_err().message,
        "refund_amount_exceeds_balance",
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
