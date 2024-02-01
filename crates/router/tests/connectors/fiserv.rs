use std::str::FromStr;

use masking::Secret;
use router::types::{self, api, storage::enums};
use serde_json::json;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct FiservTest;
impl ConnectorActions for FiservTest {}
impl utils::Connector for FiservTest {
        /// This method returns the connector data for the Fiserv connector,
    /// including the connector type, name, token type, and merchant connector ID.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Fiserv;
        types::api::ConnectorData {
            connector: Box::new(&Fiserv),
            connector_name: types::Connector::Fiserv,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// Retrieves the authentication token for the connector.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .fiserv
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "fiserv".
    fn get_name(&self) -> String {
        "fiserv".to_string()
    }
        /// Retrieves the meta information of the connector.
    ///
    /// # Returns
    ///
    /// An Option containing the serde_json Value representing the connector's meta information.
    ///
    fn get_connector_meta(&self) -> Option<serde_json::Value> {
        Some(json!({"terminalId": "10000001"}))
    }
}

/// Retrieves the details of the payment method, including the card number, expiration date, cardholder name, CVC, and capture method.
fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: cards::CardNumber::from_str("4005550000000019").unwrap(),
            card_exp_month: Secret::new("02".to_string()),
            card_exp_year: Secret::new("2035".to_string()),
            card_holder_name: Some(masking::Secret::new("John Doe".to_string())),
            card_cvc: Secret::new("123".to_string()),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(masking::Secret::new("nick_name".into())),
        }),
        capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
        ..utils::PaymentAuthorizeType::default().0
    })
}

/// Retrieves the default payment information, including the connector meta data with terminal ID "10000001".
fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        connector_meta_data: Some(json!({"terminalId": "10000001"})),
        ..Default::default()
    })
}

static CONNECTOR: FiservTest = FiservTest {};

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[serial_test::serial]
/// Asynchronously authorizes a payment using the payment method details and default payment information.
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[serial_test::serial]
/// Asynchronously attempts to capture an authorized payment using the CONNECTOR.
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(payment_method_details(), None, get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[serial_test::serial]
/// Asynchronously initiates an authorized payment capture for the specified amount using the CONNECTOR's authorize_and_capture_payment method. 
/// It captures a specified amount from the authorized payment and expects a response with status "Charged" if successful.
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
#[serial_test::serial]
/// Asynchronously authorizes a payment and waits for the payment to reach the authorized status.
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
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
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[serial_test::serial]
#[ignore]
/// Asynchronously authorizes and voids a payment using the CONNECTOR. 
/// It uses the provided payment method details, cancellation reason, and default payment info to authorize and void the payment. 
/// It then expects a void payment response and asserts that the response status is voided.
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            payment_method_details(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("TIMEOUT".to_string()),
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
#[serial_test::serial]
/// Asynchronously captures a payment and refunds it manually if necessary.
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
#[serial_test::serial]
/// Asynchronously captures a payment and partially refunds the captured amount manually.
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
#[serial_test::serial]
/// Asynchronously captures a payment and processes a refund manually, then waits for the refund status to be successful.
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
    tokio::time::sleep(std::time::Duration::from_secs(7)).await;
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
#[serial_test::serial]
/// Asynchronously makes a payment using the CONNECTOR service, with the provided payment method details and default payment information. 
/// It awaits the response and asserts that the authorize response status is equal to enums::AttemptStatus::Charged. 
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[serial_test::serial]
/// Asynchronously checks if the auto-captured payment should be synchronized. It makes a payment using the CONNECTOR, then waits for 7 seconds before retrying to synchronize the payment status. It asserts that the authorize response status is 'Charged' and that the connector transaction id is not empty. Finally, it asserts that the synchronized response status is also 'Charged'.
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    tokio::time::sleep(std::time::Duration::from_secs(7)).await;
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[serial_test::serial]
/// Asynchronously makes a payment and then attempts to refund the payment using the default payment information. 
/// This method checks if the refund status is successful and asserts the result.
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
#[serial_test::serial]
/// Asynchronously makes a partial refund for a succeeded payment. It uses the CONNECTOR to make the payment and refund, passing in the payment method details, refund amount, and payment information. The method then awaits the refund response and unwraps it to get the refund status, asserting that it is a success.
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
#[serial_test::serial]
/// Asynchronously makes a payment and initiates multiple refunds for a succeeded payment. 
/// This method uses the CONNECTOR to make the payment and initiate multiple refunds using the provided payment method details, refund amount, and payment information.
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
#[serial_test::serial]
/// Asynchronously checks if a refund should be synced with the payment gateway.
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(payment_method_details(), None, get_default_payment_info())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(7)).await;
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
// Creates a payment with incorrect card number.
#[actix_web::test]
#[serial_test::serial]
/// Attempts to make a payment with an incorrect card number and expects the payment to fail with a specific error message.
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
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Unable to assign card to brand: Invalid.".to_string(),
    );
}

// Creates a payment with incorrect CVC.
#[actix_web::test]
#[serial_test::serial]
/// Asynchronously makes a payment with incorrect CVC and asserts that the payment fails with an error message indicating invalid or missing field data.
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
        "Invalid or Missing Field Data".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
#[serial_test::serial]
/// Asynchronously makes a payment and checks if the payment fails for an invalid expiration month.
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
        "Invalid or Missing Field Data".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
#[serial_test::serial]
/// Asynchronously makes a payment with incorrect expiry year and asserts that the payment fails with the expected error message.
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
        "Unable to assign card to brand: Invalid.".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
#[serial_test::serial]
#[ignore]
/// This method attempts to make a payment, then voids the payment and asserts that the voiding fails for an auto-captured payment.
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(
            txn_id.unwrap(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("TIMEOUT".to_string()),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "You cannot cancel this PaymentIntent because it has a status of succeeded."
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
#[serial_test::serial]
/// Asynchronously attempts to capture a payment and expects the operation to fail with an error message indicating that the referenced transaction is invalid or not found.
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("Referenced transaction is invalid or not found")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
#[serial_test::serial]
/// Asynchronously makes a payment and attempts to refund an amount higher than the original payment amount. Expects the payment method details, refund data with a higher refund amount, and default payment information. 
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
        "Unable to Refund: Amount is greater than original transaction",
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
