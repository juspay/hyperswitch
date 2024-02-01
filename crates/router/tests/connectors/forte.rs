use std::{str::FromStr, time::Duration};

use cards::CardNumber;
use masking::Secret;
use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct ForteTest;
impl ConnectorActions for ForteTest {}
impl utils::Connector for ForteTest {
        /// Retrieves the connector data by constructing a new instance of `types::api::ConnectorData`.
    /// The method initializes the `connector` field with a boxed reference to the `Forte` connector,
    /// sets the `connector_name` field to `types::Connector::Forte`, and specifies the `get_token`
    /// field as `types::api::GetToken::Connector`. The `merchant_connector_id` field is set to `None`.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Forte;
        types::api::ConnectorData {
            connector: Box::new(&Forte),
            connector_name: types::Connector::Forte,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// Retrieves the authentication token for the connector.
    /// 
    /// This method retrieves the authentication token for the connector by creating a new ConnectorAuthentication instance and converting it to the appropriate ConnectorAuthType using the to_connector_auth_type utility function.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .forte
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "forte".
    fn get_name(&self) -> String {
        "forte".to_string()
    }
}

static CONNECTOR: ForteTest = ForteTest {};

/// Retrieves payment data for authorization, returning it as an Option. If payment data is available, it is wrapped in Some, otherwise None is returned.
fn get_payment_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: CardNumber::from_str("4111111111111111").unwrap(),
            ..utils::CCardType::default().0
        }),
        ..utils::PaymentAuthorizeType::default().0
    })
}

/// Retrieves the default payment information, including address and phone details.
fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        address: Some(types::PaymentAddress {
            billing: Some(api::Address {
                address: Some(api::AddressDetails {
                    first_name: Some(Secret::new("first".to_string())),
                    last_name: Some(Secret::new("last".to_string())),
                    line1: Some(Secret::new("line1".to_string())),
                    line2: Some(Secret::new("line2".to_string())),
                    city: Some("city".to_string()),
                    zip: Some(Secret::new("zip".to_string())),
                    country: Some(api_models::enums::CountryAlpha2::IN),
                    ..Default::default()
                }),
                phone: Some(api::PhoneDetails {
                    number: Some(Secret::new("1234567890".to_string())),
                    country_code: Some("+91".to_string()),
                }),
            }),
            ..Default::default()
        }),
        ..Default::default()
    })
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment using the CONNECTOR, then asserts that the response status is 'Authorized'.
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures an authorized payment by first authorizing the payment through the CONNECTOR,
/// then capturing the payment using the transaction ID and default payment information. 
/// The method expects the authorization and capture payment responses to be successful and asserts that 
/// the status of the payment is in Pending state.
async fn should_capture_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
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
    //Status of the Payments is always in Pending State, Forte has to settle the sandbox transaction manually
    assert_eq!(response.status, enums::AttemptStatus::Pending);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously performs an authorized payment capture with a partial amount.
async fn should_partially_capture_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
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
    //Status of the Payments is always in Pending State, Forte has to settle the sandbox transactions manually
    assert_eq!(response.status, enums::AttemptStatus::Pending);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment and synchronizes the authorized payment. 
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
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
                sync_type: types::SyncRequestType::SinglePaymentSync,
                connector_meta: None,
                mandate_id: None,
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously voids an authorized payment by first authorizing the payment through the connector, then voiding the payment based on the authorization response. If the void payment response does not include a status, it sets the status to pending for later synchronization.
async fn should_void_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
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
            None,
        )
        .await
        .expect("Void payment response");
    //Forte doesnot send status in response, so setting it to pending so later it will be synced
    assert_eq!(response.status, enums::AttemptStatus::Pending);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Since Payment status is always in pending, cannot refund"]
/// Initiates a manual refund process for a payment that was captured manually using the CONNECTOR. 
/// This method first authorizes the payment, then captures the payment, and finally refunds the payment. 
/// It asserts that the refund status is "Success" after completing the refund.
async fn should_refund_manually_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), None)
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    let capture_connector_meta = utils::get_connector_metadata(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    let refund_connector_metadata = utils::get_connector_metadata(capture_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id,
            Some(types::RefundsData {
                connector_metadata: refund_connector_metadata,
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

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Since Payment status is always in pending, cannot refund"]
/// Asynchronously initiates a partial refund for a manually captured payment. 
async fn should_partially_refund_manually_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), None)
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    let capture_connector_meta = utils::get_connector_metadata(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    let refund_connector_metadata = utils::get_connector_metadata(capture_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id,
            Some(types::RefundsData {
                connector_metadata: refund_connector_metadata,
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

// Synchronizes a refund using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Since Payment status is always in pending, cannot refund"]
/// Asynchronously captures a payment and refunds it, then waits for the refund status to be successful.
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(None, None, None, get_default_payment_info())
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
/// Asynchronously makes a payment using the CONNECTOR with the payment data and default payment information. 
/// The status of the payment is always in the Pending state, as Forte has to settle the sandbox transaction manually. 
async fn should_make_payment() {
    let response = CONNECTOR
        .make_payment(get_payment_data(), get_default_payment_info())
        .await
        .unwrap();
    //Status of the Payments is always in Pending State, Forte has to settle the sandbox transaction manually
    assert_eq!(response.status, enums::AttemptStatus::Pending);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously checks if an auto-captured payment should be synced. It makes a payment using the CONNECTOR, waits for the response to be in Pending status, retrieves the connector transaction id, and then retries syncing the payment until the status matches Charged. Finally, it asserts that the response status is Pending.
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
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
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    //Status of the Payments is always in Pending State, Forte has to settle the sandbox transaction manually
    assert_eq!(response.status, enums::AttemptStatus::Pending,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Since Payment status is always in pending, cannot refund"]
/// Asynchronously refunds a payment that was auto-captured by the connector after making the payment and waiting for 10 seconds. 
async fn should_refund_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), get_default_payment_info())
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    tokio::time::sleep(Duration::from_secs(10)).await;
    let refund_connector_metadata = utils::get_connector_metadata(authorize_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id,
            Some(types::RefundsData {
                connector_metadata: refund_connector_metadata,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Pending,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Since Payment status is always in pending, cannot refund"]
/// Asynchronously initiates a partial refund for a previously succeeded payment. 
/// It first makes a payment using the CONNECTOR with the provided payment data and default payment info, 
/// then retrieves the transaction ID from the authorization response, waits for 10 seconds, 
/// retrieves connector metadata from the authorization response, and finally refunds a specific amount 
/// (in this case 50) of the payment using the retrieved transaction ID, refund amount, 
/// connector metadata, and default payment info. It then asserts that the refund status is pending.
async fn should_partially_refund_succeeded_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), get_default_payment_info())
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    tokio::time::sleep(Duration::from_secs(10)).await;
    let refund_connector_metadata = utils::get_connector_metadata(authorize_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id,
            Some(types::RefundsData {
                refund_amount: 50,
                connector_metadata: refund_connector_metadata,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Pending,
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Since Payment status is always in pending, cannot refund"]
/// Asynchronously makes a payment using the CONNECTOR, retrieves the transaction ID from the authorize response, waits for 10 seconds, and then attempts to refund the payment twice using the transaction ID and refund details. It asserts that the refund status is Pending for both refund attempts.
async fn should_refund_succeeded_payment_multiple_times() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), get_default_payment_info())
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    tokio::time::sleep(Duration::from_secs(10)).await;

    let refund_connector_metadata = utils::get_connector_metadata(authorize_response.response);
    for _x in 0..2 {
        let refund_response = CONNECTOR
            .refund_payment(
                txn_id.clone(),
                Some(types::RefundsData {
                    connector_metadata: refund_connector_metadata.clone(),
                    refund_amount: 50,
                    ..utils::PaymentRefundType::default().0
                }),
                get_default_payment_info(),
            )
            .await
            .unwrap();
        assert_eq!(
            refund_response.response.unwrap().refund_status,
            enums::RefundStatus::Pending,
        );
    }
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Since Payment status is always in pending, cannot refund"]
/// Asynchronously performs a series of actions to process a refund, including making a payment, retrieving transaction ID, waiting for a specified duration, and then processing the refund payment. It also synchronously retries until the refund status matches the specified status, and asserts that the response refund status is pending.
async fn should_sync_refund() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), get_default_payment_info())
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    tokio::time::sleep(Duration::from_secs(10)).await;
    let refund_connector_metadata = utils::get_connector_metadata(authorize_response.response);
    let refund_response = CONNECTOR
        .refund_payment(
            txn_id,
            Some(types::RefundsData {
                connector_metadata: refund_connector_metadata,
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
        enums::RefundStatus::Pending,
    );
}

// Cards Negative scenerios
// Creates a payment with incorrect CVC.
#[actix_web::test]
/// Asynchronously makes a payment with incorrect CVC and asserts that the payment fails with an "INVALID CVV DATA" error message.
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
        "INVALID CVV DATA".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// This method tests if a payment fails for an invalid expiration month of a credit card.
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
        "INVALID EXPIRATION DATE".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
#[ignore]
/// Asynchronously makes a payment using the CONNECTOR, providing payment information with an incorrect expiry year, and expects the payment to fail with a specific error message related to the expiration year.
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
        "Your card's expiration year is invalid.".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
/// Attempts to fail a void payment for an auto-capture transaction.
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_payment_data(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
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
        void_response.response.unwrap_err().message,
        "ORIG TRANS NOT FOUND"
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// Asynchronously attempts to capture a payment and expects the capture to fail due to an invalid payment. 
async fn should_fail_capture_for_invalid_payment() {
    let connector_meta = Some(serde_json::json!({
        "auth_id": "56YH8TZ",
    }));
    let capture_response = CONNECTOR
        .capture_payment(
            "123456789".to_string(),
            Some(types::PaymentsCaptureData {
                connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        "Error[1]: The value for field transaction_id is invalid. Check for possible formatting issues. Error[2]: The value for field transaction_id is invalid. Check for possible formatting issues.",
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
#[ignore]
/// Asynchronously tests that a refund request fails when the refund amount is higher than the payment amount.
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let authorize_response = CONNECTOR
        .make_payment(get_payment_data(), None)
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone()).unwrap();
    tokio::time::sleep(Duration::from_secs(10)).await;
    let refund_connector_metadata = utils::get_connector_metadata(authorize_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id,
            Some(types::RefundsData {
                refund_amount: 1500,
                connector_metadata: refund_connector_metadata,
                ..utils::PaymentRefundType::default().0
            }),
            None,
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

// Cards Negative scenerios
// Creates a payment with incorrect card issuer.

#[actix_web::test]
/// This method is used to test that a specific error is thrown when a payment method is not supported by the connector.
async fn should_throw_not_implemented_for_unsupported_issuer() {
    let authorize_data = Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: CardNumber::from_str("6759649826438453").unwrap(),
            ..utils::CCardType::default().0
        }),
        capture_method: Some(enums::CaptureMethod::Automatic),
        ..utils::PaymentAuthorizeType::default().0
    });
    let response = CONNECTOR.make_payment(authorize_data, None).await;
    assert_eq!(
        *response.unwrap_err().current_context(),
        router::core::errors::ConnectorError::NotSupported {
            message: "Maestro".to_string(),
            connector: "Forte",
        }
    )
}
