use std::{str::FromStr, time::Duration};

use masking::Secret;
use router::types::{
    self, api,
    storage::{self, enums},
    PaymentsResponseData,
};
use test_utils::connector_auth::ConnectorAuthentication;

use crate::utils::{self, get_connector_transaction_id, Connector, ConnectorActions};

#[derive(Clone, Copy)]
struct SquareTest;
impl ConnectorActions for SquareTest {}
impl Connector for SquareTest {
        /// This method returns the data for a Square connector, including the connector itself, the connector name, the method for getting the connector's token, and the merchant connector ID.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Square;
        types::api::ConnectorData {
            connector: Box::new(&Square),
            connector_name: types::Connector::Square,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// This method retrieves the authentication token for the connector. It creates a new ConnectorAuthentication instance, accesses the square field and expects a value, then converts it into the appropriate ConnectorAuthType using the utils::to_connector_auth_type method.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            ConnectorAuthentication::new()
                .square
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "square" as a String.
    fn get_name(&self) -> String {
        "square".to_string()
    }
}

static CONNECTOR: SquareTest = SquareTest {};

/// Returns the default payment information with the provided payment method token, if any.
fn get_default_payment_info(payment_method_token: Option<String>) -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        address: None,
        auth_type: None,
        access_token: None,
        connector_meta_data: None,
        return_url: None,
        connector_customer: None,
        payment_method_token,
        payout_method_data: None,
        currency: None,
        country: None,
    })
}

/// Returns the details of the payment method, if available.
///
/// This method retrieves the details of the payment method used for authorization. If the details
/// are available, it returns Some(PaymentsAuthorizeData), otherwise it returns None.
fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    None
}

/// Retrieves token details for a payment method, if available.
fn token_details() -> Option<types::PaymentMethodTokenizationData> {
    Some(types::PaymentMethodTokenizationData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
            card_exp_month: Secret::new("04".to_string()),
            card_exp_year: Secret::new("2027".to_string()),
            card_cvc: Secret::new("100".to_string()),
            ..utils::CCardType::default().0
        }),
        browser_info: None,
        amount: None,
        currency: storage::enums::Currency::USD,
    })
}

/// Asynchronously creates a payment token using the default payment information. 
/// If successful, returns the payment token as a `String`, otherwise returns `None`.
async fn create_token() -> Option<String> {
    let token_response = CONNECTOR
        .create_connector_pm_token(token_details(), get_default_payment_info(None))
        .await
        .expect("Authorize payment response");
    match token_response.response.unwrap() {
        PaymentsResponseData::TokenizationResponse { token } => Some(token),
        _ => None,
    }
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment using the CONNECTOR and asserts that the response status is 'Authorized'.
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(
            payment_method_details(),
            get_default_payment_info(create_token().await),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures an authorized payment if the user is authorized, using the provided payment method details and default payment information.
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            payment_method_details(),
            None,
            get_default_payment_info(create_token().await),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not support partial capture"]
/// Asynchronously authorizes and partially captures a payment using the CONNECTOR.
///
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            payment_method_details(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(create_token().await),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment, retrieves the transaction ID from the authorization response,
/// and then retries syncing the payment until the status matches the authorized status. Asserts
/// that the response status is authorized.
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            payment_method_details(),
            get_default_payment_info(create_token().await),
        )
        .await
        .expect("Authorize payment response");
    let txn_id = get_connector_transaction_id(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(None),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes and voids a payment using the connector. 
/// 
/// This method calls the authorize_and_void_payment function of the CONNECTOR, passing in the payment method details, cancellation data and default payment info. It then awaits the response and expects a void payment response, asserting that the response status is Voided.
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            payment_method_details(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            get_default_payment_info(create_token().await),
        )
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures a payment and processes a refund for the captured payment.
async fn should_refund_manually_captured_payment() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
            None,
            None,
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None),
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
/// Asynchronously performs a partial refund for a manually captured payment using the CONNECTOR.
async fn should_partially_refund_manually_captured_payment() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None),
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
/// Asynchronously captures a payment and processes a refund, then retries syncing until the refund status matches the success status.
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
            None,
            None,
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None),
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
/// Asynchronously makes a payment by calling the `make_payment` method of the `CONNECTOR` object with the provided payment method details and default payment information obtained from creating a token. It then unwraps the result, expecting a successful payment response, and asserts that the status of the response is equal to `enums::AttemptStatus::Charged`.
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            payment_method_details(),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment using the CONNECTOR, checks the authorize response status, retrieves the transaction ID, and then retries syncing with the CONNECTOR until the status matches enums::AttemptStatus::Charged with a capture method of enums::CaptureMethod::Automatic.
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            payment_method_details(),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = get_connector_transaction_id(authorize_response.response);
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
            get_default_payment_info(None),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a refund for an automatically captured payment and ensures that the refund is successful. 
async fn should_refund_auto_captured_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            None,
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a partial refund for a succeeded payment. 
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to refund a previously successful payment multiple times.
async fn should_refund_succeeded_payment_multiple_times() {
    //make a successful payment
    let response = CONNECTOR
        .make_payment(
            payment_method_details(),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let refund_data = Some(types::RefundsData {
        refund_amount: 50,
        ..utils::PaymentRefundType::default().0
    });
    //try refund for previous payment
    let transaction_id = get_connector_transaction_id(response.response).unwrap();
    for _x in 0..2 {
        tokio::time::sleep(Duration::from_secs(CONNECTOR.get_request_interval())).await; // to avoid 404 error
        let refund_response = CONNECTOR
            .refund_payment(
                transaction_id.clone(),
                refund_data.clone(),
                get_default_payment_info(None),
            )
            .await
            .unwrap();
        let response = CONNECTOR
            .rsync_retry_till_status_matches(
                enums::RefundStatus::Success,
                refund_response.response.unwrap().connector_refund_id,
                None,
                get_default_payment_info(None),
            )
            .await
            .unwrap();
        assert_eq!(
            response.response.unwrap().refund_status,
            enums::RefundStatus::Success,
        );
    }
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment and then attempts to sync the refund status till it matches the specified status.
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            None,
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(None),
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
/// This method is used to test if a payment fails when an incorrect CVC (Card Verification Code) is provided. 
///
async fn should_fail_payment_for_incorrect_cvc() {
    let token_response = CONNECTOR
        .create_connector_pm_token(
            Some(types::PaymentMethodTokenizationData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
                    card_exp_month: Secret::new("11".to_string()),
                    card_exp_year: Secret::new("2027".to_string()),
                    card_cvc: Secret::new("".to_string()),
                    ..utils::CCardType::default().0
                }),
                browser_info: None,
                amount: None,
                currency: storage::enums::Currency::USD,
            }),
            get_default_payment_info(None),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(
        token_response
            .response
            .unwrap_err()
            .reason
            .unwrap_or("".to_string()),
        "Missing required parameter.".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// This method tests whether a payment should fail for an invalid expiration month on a card.
async fn should_fail_payment_for_invalid_exp_month() {
    let token_response = CONNECTOR
        .create_connector_pm_token(
            Some(types::PaymentMethodTokenizationData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
                    card_exp_month: Secret::new("20".to_string()),
                    card_exp_year: Secret::new("2027".to_string()),
                    card_cvc: Secret::new("123".to_string()),
                    ..utils::CCardType::default().0
                }),
                browser_info: None,
                amount: None,
                currency: storage::enums::Currency::USD,
            }),
            get_default_payment_info(None),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(
        token_response
            .response
            .unwrap_err()
            .reason
            .unwrap_or("".to_string()),
        "Invalid card expiration date.".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
/// This method tests the functionality of failing a payment for an incorrect expiry year on a card by creating a payment method token with a card data containing an incorrect expiry year, and then asserting that the response contains an error with the reason "Invalid card expiration date.".
async fn should_fail_payment_for_incorrect_expiry_year() {
    let token_response = CONNECTOR
        .create_connector_pm_token(
            Some(types::PaymentMethodTokenizationData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
                    card_exp_month: Secret::new("11".to_string()),
                    card_exp_year: Secret::new("2000".to_string()),
                    card_cvc: Secret::new("123".to_string()),
                    ..utils::CCardType::default().0
                }),
                browser_info: None,
                amount: None,
                currency: storage::enums::Currency::USD,
            }),
            get_default_payment_info(None),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(
        token_response
            .response
            .unwrap_err()
            .reason
            .unwrap_or("".to_string()),
        "Invalid card expiration date.".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to fail a void payment for auto-capture, by making a payment with the provided payment method details and default payment information, then attempting to void the payment using the connector transaction ID obtained from the payment response. It asserts that the authorize response status is 'Charged' and that the connector transaction ID is not empty, then asserts that the void response contains an error related to the invalid state of the payment.
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(
            payment_method_details(),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(
            txn_id.clone().unwrap(),
            None,
            get_default_payment_info(None),
        )
        .await
        .unwrap();
    let connector_transaction_id = txn_id.unwrap();
    assert_eq!(
        void_response.response.unwrap_err().reason.unwrap_or("".to_string()),
        format!("Payment {connector_transaction_id} is in inflight state COMPLETED, which is invalid for the requested operation")
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// Asynchronously attempts to capture a payment with an invalid payment id and expects it to fail with a specific error message.
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment(
            "123456789".to_string(),
            None,
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    assert_eq!(
        capture_response
            .response
            .unwrap_err()
            .reason
            .unwrap_or("".to_string()),
        String::from("Could not find payment with id: 123456789")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
/// Asynchronously makes a payment and refund, and asserts that the refund amount is higher than the payment amount, causing the method to fail with the appropriate error message.
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(create_token().await),
        )
        .await
        .unwrap();
    assert_eq!(
        response
            .response
            .unwrap_err()
            .reason
            .unwrap_or("".to_string()),
        "The requested refund amount exceeds the amount available to refund.",
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
