use std::str::FromStr;

use masking::Secret;
use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct AuthorizedotnetTest;
impl ConnectorActions for AuthorizedotnetTest {}
impl utils::Connector for AuthorizedotnetTest {
        /// Retrieves the data for the Authorizedotnet connector.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Authorizedotnet;
        types::api::ConnectorData {
            connector: Box::new(&Authorizedotnet),
            connector_name: types::Connector::Authorizedotnet,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// Retrieves the authentication token for the connector.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .authorizedotnet
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// This method returns the name "authorizedotnet" as a String.
    fn get_name(&self) -> String {
        "authorizedotnet".to_string()
    }
}
static CONNECTOR: AuthorizedotnetTest = AuthorizedotnetTest {};

/// Retrieves payment method data for a card transaction.
fn get_payment_method_data() -> api::Card {
    api::Card {
        card_number: cards::CardNumber::from_str("5424000000000015").unwrap(),
        card_exp_month: Secret::new("02".to_string()),
        card_exp_year: Secret::new("2035".to_string()),
        card_holder_name: Some(masking::Secret::new("John Doe".to_string())),
        card_cvc: Secret::new("123".to_string()),
        ..Default::default()
    }
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// This method is responsible for authorizing a payment using the CONNECTOR. It first attempts to authorize the payment with the specified amount, payment method data, and capture method. Then it waits for the authorization status to change to 'Authorized' using the psync_retry_till_status_matches method. If successful, it asserts that the authorize response status is 'Pending' and the psync response status is 'Authorized'.
async fn should_only_authorize_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 300,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id =
        utils::get_connector_transaction_id(authorize_response.response).unwrap_or_default();
    let psync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");

    assert_eq!(psync_response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures an authorized payment by performing a series of API calls including authorization, synchronization, and capture. 
async fn should_capture_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 301,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id =
        utils::get_connector_transaction_id(authorize_response.response).unwrap_or_default();
    let psync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.clone(),
                ),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(psync_response.status, enums::AttemptStatus::Authorized);
    let cap_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 301,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    assert_eq!(cap_response.status, enums::AttemptStatus::Pending);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::CaptureInitiated,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to partially capture an authorized payment by first authorizing the payment, then synchronizing the status until it is authorized, capturing a specified amount, and finally synchronizing the status until the capture is initiated.
async fn should_partially_capture_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 302,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id =
        utils::get_connector_transaction_id(authorize_response.response).unwrap_or_default();
    let psync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.clone(),
                ),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(psync_response.status, enums::AttemptStatus::Authorized);
    let cap_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 150,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    assert_eq!(cap_response.status, enums::AttemptStatus::Pending);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::CaptureInitiated,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously performs the necessary steps to synchronize an authorized payment. This method first authorizes a payment with the specified amount, payment method data, and capture method. It then retries the synchronization process until the status matches the authorized status. Finally, it asserts that the response status is authorized.
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 303,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id =
        utils::get_connector_transaction_id(authorize_response.response).unwrap_or_default();
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).x
#[actix_web::test]
/// Asynchronously voids an authorized payment by performing the following steps:
/// 1. Authorizes the payment with the specified amount, payment method data, and capture method.
/// 2. Checks if the authorization response status is pending and retrieves the transaction ID.
/// 3. Synchronizes the payment status until it matches the authorized status using the transaction ID.
/// 4. Voids the payment with the specified transaction ID and amount.
///
async fn should_void_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 304,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id =
        utils::get_connector_transaction_id(authorize_response.response).unwrap_or_default();
    let psync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.clone(),
                ),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");

    assert_eq!(psync_response.status, enums::AttemptStatus::Authorized);
    let void_response = CONNECTOR
        .void_payment(
            txn_id,
            Some(types::PaymentsCancelData {
                amount: Some(304),
                ..utils::PaymentCancelType::default().0
            }),
            None,
        )
        .await
        .expect("Void response");
    assert_eq!(void_response.status, enums::AttemptStatus::VoidInitiated)
}

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment and ensures that the payment is initiated and pending. It then retrieves the transaction ID and retries the payment until the capture is initiated.
async fn should_make_payment() {
    let cap_response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 310,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(cap_response.status, enums::AttemptStatus::Pending);
    let txn_id = utils::get_connector_transaction_id(cap_response.response).unwrap_or_default();
    let psync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::CaptureInitiated,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.clone(),
                ),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(
        psync_response.status,
        enums::AttemptStatus::CaptureInitiated
    );
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously performs the necessary steps to sync an auto-captured payment. 
/// This method initiates a payment, checks the status, retrieves the transaction ID, 
/// and retries syncing until the status matches. It then asserts that the response status 
/// is 'CaptureInitiated'.
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 311,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                encoded_data: None,
                capture_method: None,
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously checks if a refund should be synchronized by repeatedly retrying until the refund status matches the expected value "Success" for a given refund ID. If the refund status matches "Success", the method returns the response containing the refund status. If the refund status does not match "Success", the method will continue retrying until it does. 
async fn should_sync_refund() {
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            "60217566768".to_string(),
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

// Creates a payment with empty card number.
#[actix_web::test]
/// Asynchronously tests that a payment should fail for an empty card number by making a payment with an empty card number and asserting that the response contains an error message indicating that the card number is invalid.
async fn should_fail_payment_for_empty_card_number() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("").unwrap(),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(
        x.message,
        "The 'AnetApi/xml/v1/schema/AnetApiSchema.xsd:cardNumber' element is invalid - The value XX is invalid according to its datatype 'String' - The actual length is less than the MinLength value.",
    );
}

// Creates a payment with incorrect CVC.
#[actix_web::test]
/// Asynchronously makes a payment using the card's CVC, and asserts that the payment fails with the correct error message if the CVC is incorrect.
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
        "The 'AnetApi/xml/v1/schema/AnetApiSchema.xsd:cardCode' element is invalid - The value XXXXXXX is invalid according to its datatype 'AnetApi/xml/v1/schema/AnetApiSchema.xsd:cardCode' - The actual length is greater than the MaxLength value.".to_string(),
    );
}
// todo()

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// Asynchronously makes a payment with an invalid expiration month and asserts that it fails with the expected error message.
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
        "Credit card expiration date is invalid.".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
/// Asynchronously makes a payment and checks if it fails for an incorrect expiry year of the card.
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
        "The credit card has expired.".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
/// Performs a void payment for an auto-capture transaction that should fail, and validates the error message returned. 
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 307,
                payment_method_data: types::api::PaymentMethodData::Card(get_payment_method_data()),
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, None)
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "The 'AnetApi/xml/v1/schema/AnetApiSchema.xsd:amount' element is invalid - The value &#39;&#39; is invalid according to its datatype 'http://www.w3.org/2001/XMLSchema:decimal' - The string &#39;&#39; is not a valid Decimal value."
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// Asynchronously attempts to capture a payment using the CONNECTOR, expecting it to fail for an invalid payment. 
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        "The transaction cannot be found."
    );
}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
/// Asynchronously processes a partial refund for a manually captured payment.
async fn should_partially_refund_manually_captured_payment() {
    // Implementation details here
}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
/// Asynchronously checks if a manually captured payment should be refunded.
async fn should_refund_manually_captured_payment() {
    // Method implementation goes here
}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
/// Asynchronously checks if a manually captured refund should be synced.
async fn should_sync_manually_captured_refund() {
    // Method implementation goes here
}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
/// Asynchronously checks if a payment that was auto-captured should be refunded.
async fn should_refund_auto_captured_payment() {
    // Method implementation goes here
}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
/// Asynchronously handles the process of partially refunding a succeeded payment.
async fn should_partially_refund_succeeded_payment() {
    // method implementation goes here
}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
/// Asynchronously processes multiple successful payment refunds.
/// This method checks for multiple successful payments and initiates the refund process for each one.
async fn should_refund_succeeded_payment_multiple_times() {
    // implementation goes here
}

#[actix_web::test]
#[ignore = "refunds tests are ignored for this connector because it takes one day for a payment to be settled."]
/// Asynchronously checks if the refund amount is higher than the payment amount.
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    // method implementation
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
