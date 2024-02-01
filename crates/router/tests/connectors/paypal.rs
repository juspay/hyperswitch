use std::str::FromStr;

use masking::Secret;
use router::types::{self, api, storage::enums, AccessToken, ConnectorAuthType};

use crate::{
    connector_auth,
    utils::{self, Connector, ConnectorActions},
};

struct PaypalTest;
impl ConnectorActions for PaypalTest {}
impl Connector for PaypalTest {
        /// Returns the connector data for the Paypal connector.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Paypal;
        types::api::ConnectorData {
            connector: Box::new(&Paypal),
            connector_name: types::Connector::Paypal,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// This method retrieves the authentication token for the connector. It converts the connector
    /// authentication configuration into a `ConnectorAuthType` enum using the `utils::to_connector_auth_type`
    /// function and returns it.
    fn get_auth_token(&self) -> ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .paypal
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// This method returns the name "paypal" as a String.
    fn get_name(&self) -> String {
        "paypal".to_string()
    }
}
static CONNECTOR: PaypalTest = PaypalTest {};

/// Retrieves the access token for accessing the PayPal API. 
/// If the authentication token is successfully retrieved from the connector, it is returned as an Option containing an AccessToken with the token and its expiration time. 
/// If the authentication token cannot be retrieved, None is returned.
fn get_access_token() -> Option<AccessToken> {
    let connector = PaypalTest {};

    match connector.get_auth_token() {
        ConnectorAuthType::BodyKey { api_key, key1: _ } => Some(AccessToken {
            token: api_key,
            expires: 18600,
        }),
        _ => None,
    }
}
/// Retrieves the default payment information, which includes an access token obtained using the `get_access_token` function.
///
/// Returns an `Option` containing the default payment information, or `None` if the access token could not be obtained.
fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        access_token: get_access_token(),
        ..Default::default()
    })
}

/// Retrieves payment authorization data for a payment method, specifically a credit card with a hardcoded card number.
fn get_payment_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: cards::CardNumber::from_str("4000020000000000").unwrap(),
            ..utils::CCardType::default().0
        }),
        ..utils::PaymentAuthorizeType::default().0
    })
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment using the payment data and default payment information,
/// and asserts that the response status is Authorized.
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to capture an authorized payment by first authorizing the payment using the CONNECTOR, then capturing the payment using the response from the authorization. It then asserts that the capture response status is pending.
async fn should_capture_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let connector_meta = utils::get_connector_metadata(authorize_response.response);
    let response = CONNECTOR
        .capture_payment(
            txn_id,
            Some(types::PaymentsCaptureData {
                connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Pending);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to partially capture an authorized payment by first authorizing the payment using the CONNECTOR, then capturing a specified amount of the authorized payment. If successful, the method should return a response with a status of Pending.
async fn should_partially_capture_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let connector_meta = utils::get_connector_metadata(authorize_response.response);
    let response = CONNECTOR
        .capture_payment(
            txn_id,
            Some(types::PaymentsCaptureData {
                connector_meta,
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Pending);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously initiates the process to synchronize an authorized payment by first authorizing the payment via the connector, then retrying the payment synchronization until the status matches the authorized status. The method expects to receive the response for the authorize payment and uses it to construct the payment synchronization data. It then calls the psync_retry_till_status_matches method to synchronize the payment and asserts that the response status is authorized.
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let connector_meta = utils::get_connector_metadata(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                mandate_id: None,
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                encoded_data: None,
                capture_method: None,
                sync_type: types::SyncRequestType::SinglePaymentSync,
                connector_meta,
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously voids an authorized payment by first authorizing the payment with the given payment data and default payment information. Then voids the payment using the transaction id and additional payment cancellation data, along with the default payment information. Finally, it asserts that the payment status is voided.
async fn should_void_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let connector_meta = utils::get_connector_metadata(authorize_response.response);
    let response = CONNECTOR
        .void_payment(
            txn_id,
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
                connector_meta,
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
#[ignore = "Since Payment status is in pending status, cannot refund"]
/// Asynchronously refunds a manually captured payment by authorizing the payment, capturing the payment, and then refunding the payment.
async fn should_refund_manually_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let capture_connector_meta = utils::get_connector_metadata(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id,
            Some(types::PaymentsCaptureData {
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment response");
    let refund_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let response = CONNECTOR
        .refund_payment(
            refund_txn_id,
            Some(types::RefundsData {
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

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Since Payment status is in pending status, cannot refund"]
/// Asynchronously refunds a partially captured payment after authorizing and capturing it.
async fn should_partially_refund_manually_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let capture_connector_meta = utils::get_connector_metadata(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id,
            Some(types::PaymentsCaptureData {
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment response");
    let refund_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let response = CONNECTOR
        .refund_payment(
            refund_txn_id,
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
#[ignore = "Since Payment status is in pending status, cannot refund"]
/// Asynchronously performs a series of payment-related actions including authorizing a payment, capturing the payment, refunding the payment, and then retrying the refund until the refund status matches the specified status. 
async fn should_sync_manually_captured_refund() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let capture_connector_meta = utils::get_connector_metadata(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id,
            Some(types::PaymentsCaptureData {
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment response");
    let refund_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_response = CONNECTOR
        .refund_payment(
            refund_txn_id,
            Some(types::RefundsData {
                ..utils::PaymentRefundType::default().0
            }),
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
/// Asynchronously makes a payment using the CONNECTOR, payment data, and default payment information. 
/// It then waits for the payment to be authorized and asserts that the status is pending.
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously initiates a payment with auto capture, verifies the authorization status, and retries the payment synchronization until the status matches 'Charged'.
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        authorize_response.status.clone(),
        enums::AttemptStatus::Pending
    );
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone());
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let connector_meta = utils::get_connector_metadata(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                mandate_id: None,
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                encoded_data: None,
                capture_method: Some(enums::CaptureMethod::Automatic),
                sync_type: types::SyncRequestType::SinglePaymentSync,
                connector_meta,
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Pending);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Since Payment status is in pending status, cannot refund"]
/// Asynchronously makes a payment and refunds an auto-captured payment. 
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(get_payment_data(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Since Payment status is in pending status, cannot refund"]
/// Asynchronously attempts to partially refund a succeeded payment by making a payment, extracting the transaction ID from the response, and then initiating a refund with a specified refund amount. If the refund is successful, it asserts that the refund status is "Success".
async fn should_partially_refund_succeeded_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), get_default_payment_info())
        .await
        .unwrap();

    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    let refund_response = CONNECTOR
        .refund_payment(
            txn_id,
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
#[ignore = "Since Payment status is in pending status, cannot refund"]
/// Asynchronously makes a payment, gets the transaction ID from the response, and then attempts to refund the payment twice. 
/// Each refund is for a refund amount of 50, and the method asserts that the refund status is "Success" for each refund.
async fn should_refund_succeeded_payment_multiple_times() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), get_default_payment_info())
        .await
        .unwrap();

    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    for _x in 0..2 {
        let refund_response = CONNECTOR
            .refund_payment(
                txn_id.clone(),
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
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Since Payment status is in pending status, cannot refund"]
/// Asynchronously checks if a refund should be synced by making a payment and refund, then retrying until the refund status matches the expected success status.
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(get_payment_data(), None, get_default_payment_info())
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
/// Asynchronously makes a payment request with an incorrect CVC and asserts that the response
/// contains the expected error message and reason.
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
        response.response.clone().unwrap_err().message,
        "Request is not well-formed, syntactically incorrect, or violates schema.",
    );
    assert_eq!(
        response.response.unwrap_err().reason.unwrap(),
        "description - The value of a field does not conform to the expected format., value - 12345, field - security_code;",
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// Asynchronously makes a payment with an invalid expiration month and asserts that the payment fails with the expected error message and reason.
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
        response.response.clone().unwrap_err().message,
        "Request is not well-formed, syntactically incorrect, or violates schema.",
    );
    assert_eq!(
        response.response.unwrap_err().reason.unwrap(),
        "description - The value of a field does not conform to the expected format., value - 2025-20, field - expiry;",
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
/// Handles the asynchronous process of attempting a payment with incorrect expiry year, expecting it to fail.
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
        response.response.clone().unwrap_err().message,
        "The requested action could not be performed, semantically incorrect, or failed business validation.",
    );
    assert_eq!(
        response.response.unwrap_err().reason.unwrap(),
        "description - The card is expired., field - expiry;",
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
/// This asynchronous method attempts to perform a void payment for an auto-captured payment. It first authorizes a payment, then captures the payment, and finally attempts to void the payment. It expects the void payment to fail and checks that the response contains the expected error message and reason. 
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let capture_connector_meta = utils::get_connector_metadata(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id,
            Some(types::PaymentsCaptureData {
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment response");
    let txn_id = utils::get_connector_transaction_id(capture_response.clone().response).unwrap();
    let connector_meta = utils::get_connector_metadata(capture_response.response);
    let void_response = CONNECTOR
        .void_payment(
            txn_id,
            Some(types::PaymentsCancelData {
                cancellation_reason: Some("requested_by_customer".to_string()),
                connector_meta,
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Void payment response");
    assert_eq!(
        void_response.response.clone().unwrap_err().message,
        "The requested action could not be performed, semantically incorrect, or failed business validation."
    );
    assert_eq!(
        void_response.response.unwrap_err().reason.unwrap(),
        "description - Authorization has been previously captured and hence cannot be voided. ; "
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// Asynchronously tests the capture of a payment for invalid payment data. This method constructs a connector metadata object, sends a capture request with invalid payment data to the CONNECTOR, and asserts that the response contains the expected error messages. 
async fn should_fail_capture_for_invalid_payment() {
    let connector_meta = Some(serde_json::json!({
        "authorize_id": "56YH8TZ",
        "order_id":"02569315XM5003146",
        "psync_flow":"AUTHORIZE",
    }));
    let capture_response = CONNECTOR
        .capture_payment(
            "".to_string(),
            Some(types::PaymentsCaptureData {
                connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.clone().unwrap_err().message,
        "The specified resource does not exist.",
    );
    assert_eq!(
        capture_response.response.unwrap_err().reason.unwrap(),
        "description - Specified resource ID does not exist. Please check the resource ID and try again. ; ",
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
#[ignore = "Since Payment status is in pending status, cannot refund"]
/// This method tests whether a refund request fails when the refund amount is higher than the payment amount. It first makes a payment using the CONNECTOR with the given payment data and default payment information. It then attempts to refund a payment with a refund amount of 150 and asserts that the response contains the expected error message and reason.
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), get_default_payment_info())
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    let response = CONNECTOR
        .refund_payment(
            txn_id,
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(&response.response.clone().unwrap_err().message, "The requested action could not be performed, semantically incorrect, or failed business validation.");

    assert_eq!(
        response.response.unwrap_err().reason.unwrap(),
        "description - The refund amount must be less than or equal to the capture amount that has not yet been refunded. ; ",
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
