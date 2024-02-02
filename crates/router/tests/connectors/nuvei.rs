use std::str::FromStr;

use masking::Secret;
use router::types::{
    self, api,
    storage::{self, enums},
};
use serde_json::json;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct NuveiTest;
impl ConnectorActions for NuveiTest {}
impl utils::Connector for NuveiTest {
        /// Returns the connector data for Nuvei, including the connector instance, connector name, token type, and merchant connector ID.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Nuvei;
        types::api::ConnectorData {
            connector: Box::new(&Nuvei),
            connector_name: types::Connector::Nuvei,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// Retrieves the authentication token for the connector.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .nuvei
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "nuvei" as a String.
    fn get_name(&self) -> String {
        "nuvei".to_string()
    }
}

static CONNECTOR: NuveiTest = NuveiTest {};

/// Retrieves payment data for authorization, returning an option containing the payment data if successful.
fn get_payment_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: cards::CardNumber::from_str("4444 3333 2222 1111").unwrap(),
            ..utils::CCardType::default().0
        }),
        ..utils::PaymentAuthorizeType::default().0
    })
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment using the CONNECTOR and asserts that the response status is 'Authorized'.
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_data(), None)
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to authorize and capture a payment using the CONNECTOR. 
/// It then checks the response status and asserts that it is charged.
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(get_payment_data(), None, None)
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to partially capture an authorized payment. It sends a request to the CONNECTOR to both authorize and capture a payment, specifying the amount to capture and handling the response to ensure the payment is successfully captured. If the capture is successful, it asserts that the response status is 'Charged'.
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            get_payment_data(),
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
#[actix_web::test]
/// This method is used to perform a synchronized authorization of a payment. It first authorizes the payment using the CONNECTOR, retrieves the transaction ID from the authorization response, and then synchronously retries the authorization process until the status matches the specified status. Finally, it asserts that the status of the response matches the authorized status.
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), None)
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
                connector_meta: Some(json!({
                    "session_token": authorize_response.session_token.unwrap()
                })),
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes and voids a payment with the specified payment data and cancellation reason. 
///
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            get_payment_data(),
            Some(types::PaymentsCancelData {
                cancellation_reason: Some("requested_by_customer".to_string()),
                amount: Some(100),
                currency: Some(storage::enums::Currency::USD),
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures a payment and initiates a refund process if necessary. 
/// This method uses the CONNECTOR to capture the payment and then attempts to refund it. 
/// If successful, it asserts that the refund status is 'Success'.
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(get_payment_data(), None, None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures a payment and then partially refunds it manually by calling the `capture_payment_and_refund` method of the `CONNECTOR`. The refund amount is set to 50, and the refund type is set to the default value of the `PaymentRefundType` enum. After the refund operation is completed, it asserts that the refund status in the response is `Success`.
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            get_payment_data(),
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

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment using the CONNECTOR. It retrieves payment data and calls the make_payment method with the data and None as parameters. It then awaits the response and unwraps it. Finally, it asserts that the authorize response status is Charged. 
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), None)
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously performs the necessary operations to sync an auto-captured payment.
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), None)
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
                connector_meta: Some(json!({
                    "session_token": authorize_response.session_token.unwrap()
                })),
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
/// Asynchronously makes a payment and attempts to refund an auto-captured payment.
///
/// This method uses the `make_payment_and_refund` function of the `CONNECTOR` to make a payment and then attempts to refund the payment with no specific refund amount or reason. It awaits the response and asserts that the refund status is `Success` if the operation is successful.
///
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(get_payment_data(), None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment and then partially refunds the succeeded payment for a specific refund amount.
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            get_payment_data(),
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

// Cards Negative scenerios
// Creates a payment with incorrect card number.
#[actix_web::test]
/// Asynchronously tests that a payment fails for an incorrect card number by making a payment with a card number that is known to be incorrect and asserting that the response contains an error message indicating that the card number is invalid.
async fn should_fail_payment_for_incorrect_card_number() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("1234567891011").unwrap(),
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
        "Missing or invalid CardData data. Invalid credit card number.".to_string(),
    );
}

// Creates a payment with incorrect CVC.
#[actix_web::test]
/// This method is used to test that a payment fails when an incorrect CVC is provided.
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
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "cardData.CVV is invalid".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// This method tests whether a payment fails for an invalid expiration month on a card. It makes a payment using a mocked connector with the provided invalid expiration month, and then asserts that the response contains an error message indicating an invalid expiration date.
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
        "Invalid expired date".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
/// Asynchronously makes a payment with an incorrect expiry year for the card and asserts that the payment should succeed with a 'Charged' status.
async fn should_succeed_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4000027891380961").unwrap(),
                    card_exp_year: Secret::new("2000".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..get_payment_data().unwrap()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
/// This method attempts to make a void payment for an auto-captured transaction. It first makes a payment using the CONNECTOR and checks if the payment is successfully authorized. It then retrieves the transaction ID from the authorization response and uses it to void the payment, providing a cancellation reason and amount. Finally, it asserts that the void payment was successful.
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), None)
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(
            txn_id.unwrap(),
            Some(types::PaymentsCancelData {
                cancellation_reason: Some("requested_by_customer".to_string()),
                amount: Some(100),
                currency: Some(storage::enums::Currency::USD),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(void_response.status, enums::AttemptStatus::Voided);
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// This method attempts to capture a payment with an invalid relatedTransactionId and expects the capture to fail with a specific error message.  
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("Invalid relatedTransactionId")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
/// Asynchronously makes a payment and attempts to refund an amount higher than the payment amount. 
/// It then asserts that the refund status in the response is a success.
async fn should_accept_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            get_payment_data(),
            Some(types::RefundsData {
                refund_amount: 150,
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
