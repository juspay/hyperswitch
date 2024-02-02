use std::str::FromStr;

use masking::{PeekInterface, Secret};
use router::types::{self, api, storage::enums, AccessToken};

use crate::{
    connector_auth,
    utils::{self, Connector, ConnectorActions},
};

#[derive(Clone, Copy)]
struct AirwallexTest;
impl ConnectorActions for AirwallexTest {}

static CONNECTOR: AirwallexTest = AirwallexTest {};

impl Connector for AirwallexTest {
        /// Retrieves the connector data for the Airwallex connector.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Airwallex;
        types::api::ConnectorData {
            connector: Box::new(&Airwallex),
            connector_name: types::Connector::Airwallex,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// This method retrieves the authentication token for the connector. It creates a new instance of ConnectorAuthentication and expects the airwallex authentication configuration to be present. It then converts the authentication type to the appropriate type and returns it.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .airwallex
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// This method returns the name "airwallex" as a String.
    fn get_name(&self) -> String {
        "airwallex".to_string()
    }
}

/// Retrieves the access token from the CONNECTOR by calling the get_auth_token method. 
/// If the authentication type is BodyKey, it returns an AccessToken containing the API key and the expiration time parsed from the key1 field. 
/// If the authentication type is not BodyKey, it returns None.
fn get_access_token() -> Option<AccessToken> {
    match CONNECTOR.get_auth_token() {
        types::ConnectorAuthType::BodyKey { api_key, key1 } => Some(AccessToken {
            token: api_key,
            expires: key1.peek().parse::<i64>().unwrap(),
        }),
        _ => None,
    }
}
/// Returns the default payment information, wrapped in an `Option`. The payment information includes an access token obtained from the `get_access_token` function, and default values for any other fields.
fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        access_token: get_access_token(),
        ..Default::default()
    })
}
/// Retrieves the details of a payment method, including card information, capture method, return URL, and complete authorize URL.
fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: cards::CardNumber::from_str("4035501000000008").unwrap(),
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
        router_return_url: Some("https://google.com".to_string()),
        complete_authorize_url: Some("https://google.com".to_string()),
        ..utils::PaymentAuthorizeType::default().0
    })
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously authorizes a payment using the CONNECTOR, and asserts that the response status is "Authorized".
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
/// Asynchronously authorizes and captures a payment using the CONNECTOR, with the provided payment method details and default payment information.
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
/// Asynchronously performs a partial capture of an authorized payment. It authorizes and captures the payment with the given payment method details and capture data, using the default payment information. It then asserts that the response status is 'Charged'.
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
/// Asynchronously authorizes a payment, retrieves the transaction ID from the authorize response,
/// and then retries the payment synchronization process until the status matches the authorized status.
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
/// Asynchronously authorizes and voids a payment using the CONNECTOR. 
/// It cancels the payment with the provided payment method details and cancellation reason, and returns the void payment response. 
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
// #[serial_test::serial]
#[actix_web::test]
#[ignore]
/// Asynchronously captures a payment and then refunds it manually. The method uses the CONNECTOR to capture the payment and then immediately initiates a refund. It expects the payment method details, default payment information, and optionally refund details. It then awaits the response and asserts that the refund was successful.
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
// #[serial_test::serial]
#[actix_web::test]
#[ignore]
/// Asynchronously captures a payment and refunds a portion of it manually, using the CONNECTOR. 
///
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
// #[serial_test::serial]
#[actix_web::test]
#[ignore]
/// Asynchronously captures a payment and processes a refund, then synchronously retries until the refund status matches the expected Success status.
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
/// Asynchronously makes a payment using the CONNECTOR by calling the make_payment method with the provided payment method details and default payment info. 
/// It awaits the response and asserts that the authorize response status is Charged.
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
/// Asynchronously checks if an auto-captured payment should be synced. It makes a payment using the connector, verifies the authorize response status, gets the connector transaction id, and retries syncing until the status matches the expected charged status.
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
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
// #[serial_test::serial]
#[actix_web::test]
#[ignore]
/// Asynchronously makes a payment and refunds the payment if it was auto-captured. 
/// This method uses the CONNECTOR to make a payment and then refunds the payment 
/// using the provided payment method details and default payment info. It then 
/// asserts that the refund status is successful.
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
// #[serial_test::serial]
#[actix_web::test]
#[ignore]
/// Asynchronously attempts to partially refund a succeeded payment by making a payment and then initiating a refund for a specific refund amount. This method uses the CONNECTOR to make the payment and refund, and then checks if the refund was successful by verifying the refund status in the response.
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
// #[serial_test::serial]
#[actix_web::test]
#[ignore]
/// Asynchronously makes a payment and attempts multiple refunds on the payment using the CONNECTOR. 
/// 
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
// #[serial_test::serial]
#[actix_web::test]
#[ignore]
/// Asynchronously checks if a refund should be synced. It makes a payment and refund, then retries the refund status until it matches the 'Success' status. It then asserts that the refund status is 'Success'.
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
// Creates a payment with incorrect card number.
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously makes a payment using an incorrect card number and expects the payment to fail with an "Invalid card number" message.
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
        "Invalid card number".to_string(),
    );
}

// Creates a payment with incorrect CVC.
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously tests that a payment attempt with an incorrect CVC should fail with an "Invalid card cvc" message.
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
        "Invalid card cvc".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously tests that the payment should fail for an invalid expiration month.
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
        "Invalid expiry month".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously checks if a payment fails for an incorrect expiry year of the card.
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
        "payment_method.card should not be expired".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
/// This method attempts to make a void payment for an auto-captured transaction. It first makes a payment using the specified payment method details and default payment information. It then asserts that the payment was successfully authorized and retrieves the transaction id. Next, it attempts to void the payment using the transaction id and asserts that the void operation fails with a specific error message.
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
    assert_eq!(
        void_response.response.unwrap_err().message,
        "The PaymentIntent status SUCCEEDED is invalid for operation cancel."
    );
}

// Captures a payment using invalid connector payment id.
#[serial_test::serial]
#[actix_web::test]
/// Asynchronously attempts to capture a payment for an invalid payment ID and checks that the capture attempt fails with the expected error message.
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from(
            "The requested endpoint does not exist [/api/v1/pa/payment_intents/123456789/capture]"
        )
    );
}

// Refunds a payment with refund amount higher than payment amount.
// #[serial_test::serial]
#[actix_web::test]
#[ignore]
/// Asynchronously makes a payment and refund using the CONNECTOR, then asserts that the response
/// returns an error message indicating that the refund amount is higher than the payment amount.
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
        "Refund amount (₹1.50) is greater than charge amount (₹1.00)",
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
