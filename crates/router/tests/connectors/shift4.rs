use std::str::FromStr;

use masking::Secret;
use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct Shift4Test;
impl ConnectorActions for Shift4Test {}
impl utils::Connector for Shift4Test {
        /// This method returns the ConnectorData for the Shift4 connector, including the connector object, connector name, token type, and merchant connector ID.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Shift4;
        types::api::ConnectorData {
            connector: Box::new(&Shift4),
            connector_name: types::Connector::Shift4,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// Retrieves the authentication token for the connector.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .shift4
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "shift4".
    fn get_name(&self) -> String {
        "shift4".to_string()
    }
}

static CONNECTOR: Shift4Test = Shift4Test {};

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// This method is responsible for authorizing a payment using the CONNECTOR.
/// It awaits the result of the authorization and then asserts that the status of the response is 'Authorized'.
async fn should_only_authorize_payment() {
    let response = CONNECTOR.authorize_payment(None, None).await.unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment using the CONNECTOR, and asserts that the response status is 'Charged'.
async fn should_make_payment() {
    let authorize_response = CONNECTOR.make_payment(None, None).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to capture an authorized payment using the connector. 
/// If successful, the response status should be "Charged".
async fn should_capture_authorized_payment() {
    let connector = CONNECTOR;
    let response = connector
        .authorize_and_capture_payment(None, None, None)
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to partially capture an authorized payment. It uses the provided connector to authorize and capture a payment of 50 units, and then asserts that the response status is 'Charged'.
async fn should_partially_capture_authorized_payment() {
    let connector = CONNECTOR;
    let response = connector
        .authorize_and_capture_payment(
            None,
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously checks if an authorized payment needs to be synchronized with the connector. 
async fn should_sync_authorized_payment() {
    let connector = CONNECTOR;
    let authorize_response = connector.authorize_payment(None, None).await.unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = connector
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
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously verifies and syncs an automatically captured payment with the CONNECTOR. This method first authorizes the payment, then retrieves the connector transaction ID from the response, and finally syncs the payment with the connector until the status matches the 'Charged' status.
async fn should_sync_auto_captured_payment() {
    // Authorize
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
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to void an authorized payment. This method uses the connector to authorize and void a payment with the provided cancellation reason.
async fn should_void_authorized_payment() {
    let connector = CONNECTOR;
    let response = connector
        .authorize_and_void_payment(
            None,
            Some(types::PaymentsCancelData {
                connector_transaction_id: "".to_string(),
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            None,
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Pending); //shift4 doesn't allow voiding a payment
}

// Cards Negative scenerios
// Creates a payment with incorrect card number.
#[actix_web::test]
/// Asynchronously attempts a payment with an incorrect card number and checks that it fails with the appropriate error message.
async fn should_fail_payment_for_incorrect_card_number() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4024007134364842").unwrap(),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "The card's security code failed verification.".to_string(),
    );
}

// Creates a payment with incorrect CVC.
#[actix_web::test]
/// Asynchronously makes a payment with incorrect CVC and asserts that the payment should succeed.
async fn should_succeed_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_cvc: Secret::new("asdasd".to_string()), //shift4 accept invalid CVV as it doesn't accept CVV
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// Asynchronously makes a payment request with an invalid expiration month and asserts that the response contains an error message indicating that the card's expiration month is invalid.
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
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "The card's expiration month is invalid.".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
/// Asynchronously makes a payment and verifies that it fails for an incorrect expiry year of the card.
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
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "The card has expired.".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously performs a void payment for an auto-captured transaction, ensuring that the void request is pending due to the inability to void a payment in shift4.
async fn should_fail_void_payment_for_auto_capture() {
    // Authorize
    let authorize_response = CONNECTOR.make_payment(None, None).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");

    // Void
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, None)
        .await
        .unwrap();
    assert_eq!(void_response.status, enums::AttemptStatus::Pending); //shift4 doesn't allow voiding a payment
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// Asynchronously attempts to capture a payment and verifies that it fails with an invalid payment error message.
async fn should_fail_capture_for_invalid_payment() {
    // Capture
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("Charge '123456789' does not exist")
    );
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously checks if a payment that was automatically captured should be refunded. It makes a payment and then attempts to refund it, expecting a success refund status in the response.
async fn should_refund_auto_captured_payment() {
    let connector = CONNECTOR;
    let response = connector
        .make_payment_and_refund(None, None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to manually refund a captured payment by using the connector's `auth_capture_and_refund` method. 
/// If the refund is successful, the method will return without error. Otherwise, it will panic with the corresponding error message.
async fn should_refund_manually_captured_payment() {
    let connector = CONNECTOR;
    let response = connector
        .auth_capture_and_refund(None, None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

#[actix_web::test]
/// Asynchronously attempts to partially refund a succeeded payment by making a refund request through the connector. The refund amount is set to 50, and the refund status is then checked to ensure that the refund was successful.
async fn should_partially_refund_succeeded_payment() {
    let connector = CONNECTOR;
    let refund_response = connector
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

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously initiates a partial refund for a manually captured payment using the given connector. 
/// The refund amount is set to 50, and the refund status is asserted to be 'Success' upon completion. 
async fn should_partially_refund_manually_captured_payment() {
    let connector = CONNECTOR;
    let response = connector
        .auth_capture_and_refund(
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

#[actix_web::test]
/// Asynchronously makes a payment and attempts to process multiple refunds for a successful payment. 
///
/// This method uses the `connector` to make a payment and then attempt to process multiple refunds for the payment. It passes `None` for the payment ID, a `RefundsData` struct with a refund amount of 50 and default values for other refund data, and `None` for any additional parameters. 
///
async fn should_refund_succeeded_payment_multiple_times() {
    let connector = CONNECTOR;
    connector
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

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
/// This asynchronous method tests that an error is returned when attempting to refund an amount higher than the payment amount.
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let connector = CONNECTOR;
    let response = connector
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
        "Invalid Refund data",
    );
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// This method is used to asynchronously check if a refund should be synced. It first makes a payment and refund using a connector, then retries syncing the refund status until it matches the expected success status. If the refund status matches the expected status, it asserts the success of the refund.
async fn should_sync_refund() {
    let connector = CONNECTOR;
    let refund_response = connector
        .make_payment_and_refund(None, None, None)
        .await
        .unwrap();
    let response = connector
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

// Synchronizes a refund using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously performs manual synchronization for a captured refund. It first performs an authorization, capture, and refund with the specified parameters, then retries the synchronization process until the refund status matches the expected success status. Finally, it asserts that the refund status in the response matches the expected success status.
async fn should_sync_manually_captured_refund() {
    let connector = CONNECTOR;
    let refund_response = connector
        .auth_capture_and_refund(None, None, None)
        .await
        .unwrap();
    let response = connector
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
