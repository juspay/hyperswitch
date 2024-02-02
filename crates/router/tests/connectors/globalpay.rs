use std::str::FromStr;

use masking::Secret;
use router::types::{self, api, storage::enums, AccessToken, ConnectorAuthType};
use serde_json::json;

use crate::{
    connector_auth,
    utils::{self, Connector, ConnectorActions, PaymentInfo},
};

struct Globalpay;
impl ConnectorActions for Globalpay {}
static CONNECTOR: Globalpay = Globalpay {};
impl Connector for Globalpay {
        /// Retrieves the connector data for the Globalpay connector.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Globalpay;
        types::api::ConnectorData {
            connector: Box::new(&Globalpay),
            connector_name: types::Connector::Globalpay,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// This method retrieves the authentication token for the connector.
    fn get_auth_token(&self) -> ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .globalpay
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "globalpay" as a String.
    fn get_name(&self) -> String {
        "globalpay".to_string()
    }

        /// Retrieves the metadata for the connector, if available.
    fn get_connector_meta(&self) -> Option<serde_json::Value> {
        Some(json!({"account_name": "transaction_processing"}))
    }
}

/// Retrieves the access token for authentication. 
/// If the authentication token is obtained from the connector with a body key, it returns an `AccessToken` with the token and expiration time. 
/// If the authentication token is not obtained from the connector with a body key, it returns `None`.
fn get_access_token() -> Option<AccessToken> {
    match utils::Connector::get_auth_token(&CONNECTOR) {
        ConnectorAuthType::BodyKey { api_key, key1: _ } => Some(AccessToken {
            token: api_key,
            expires: 18600,
        }),
        _ => None,
    }
}

impl Globalpay {
        /// Returns the interval for making a request in milliseconds.
    fn get_request_interval(&self) -> u64 {
        5
    }
        /// Retrieves payment information including the payment address, access token, and connector meta data.
    fn get_payment_info() -> Option<PaymentInfo> {
        Some(PaymentInfo {
            address: Some(types::PaymentAddress {
                billing: Some(api::Address {
                    address: Some(api::AddressDetails {
                        country: Some(api_models::enums::CountryAlpha2::US),
                        ..Default::default()
                    }),
                    phone: None,
                }),
                ..Default::default()
            }),
            access_token: get_access_token(),
            connector_meta_data: CONNECTOR.get_connector_meta(),
            ..Default::default()
        })
    }
}

#[actix_web::test]
/// Asynchronously authorizes a payment using the Globalpay payment information and asserts that the response status is Authorized.
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
/// Asynchronously makes a payment using the CONNECTOR and the payment information obtained from Globalpay.
/// Asserts that the response status is 'Charged'.
async fn should_make_payment() {
    let response = CONNECTOR
        .make_payment(None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
/// Asynchronously performs an authorized payment capture operation. It calls the `authorize_and_capture_payment` method on the `CONNECTOR` with the specified capture data and payment info from `Globalpay`. It then asserts that the response status is `Charged`.
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            None,
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            Globalpay::get_payment_info(),
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
/// Asynchronously checks if auto-captured payment should be synchronized. This method authorizes the payment through the CONNECTOR, retrieves the transaction ID, and then retries synchronization until the status matches the authorized status. Finally, it asserts that the response status is authorized.
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(None, Globalpay::get_payment_info())
        .await
        .unwrap();
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
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

#[actix_web::test]
/// Asynchronously makes a payment with a fake card number and expects the payment to fail due to incorrect CVC.
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4024007134364842").unwrap(),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    let x = response.status;
    assert_eq!(x, enums::AttemptStatus::Failure);
}

#[actix_web::test]
/// Asynchronously makes a payment and refunds the auto-captured payment using the Globalpay payment information. 
/// It then asserts that the refund status is successful.
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(None, None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

#[actix_web::test]
/// Asynchronously performs an authorized void payment using the CONNECTOR and asserts that the response status is voided.
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(None, None, Globalpay::get_payment_info())
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Voided);
}

#[actix_web::test]
/// Asynchronously checks if a refund should be synced by making a payment and refund request, then waiting for a specified interval to synchronize the refund status.
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(None, None, Globalpay::get_payment_info())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(
        CONNECTOR.get_request_interval(),
    ))
    .await;
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// This method asynchronously captures an authorized payment for a specified amount using the Globalpay connector. It captures a partial amount of 50 units and expects a response with a status of 'Charged', asserting that the capture payment was successful.
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            None,
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            Globalpay::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
/// Asynchronously captures a payment and partially refunds it manually.
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            None,
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            Globalpay::get_payment_info(),
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
/// Should partially refund a succeeded payment by making a payment, obtaining the transaction ID, and then refunding a specified amount using the obtained transaction ID.
async fn should_partially_refund_succeeded_payment() {
    let authorize_response = CONNECTOR
        .make_payment(None, Globalpay::get_payment_info())
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
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Synchronizes a refund using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures a payment and processes a refund, then waits for the refund to sync manually. 
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(None, None, None, Globalpay::get_payment_info())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(
        CONNECTOR.get_request_interval(),
    ))
    .await;
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
/// Asynchronously makes a payment and refund using the CONNECTOR, and asserts that it fails for a refund amount higher than the payment amount by 115%. 
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "You may only refund up to 115% of the original amount ",
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// Asynchronously attempts to capture a payment and expects it to fail for an invalid payment. It calls the `capture_payment` method on the `CONNECTOR` object with the given payment ID and no additional information. It then awaits the result and unwraps the response. It asserts that the capture response contains an error message indicating that the transaction with the given ID was not found at the specified location.
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123ddsa12".to_string(), None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("Transaction 123ddsa12 not found at this location.")
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore]
/// Asynchronously tests if a void payment for auto capture fails as expected.
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "You cannot cancel this PaymentIntent because it has a status of succeeded."
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
/// This asynchronous method attempts to make a payment using a specific payment method data with an incorrect expiry year, and then asserts that the response contains an error message indicating that the expiry date is invalid.
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
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Expiry date invalid".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// Asynchronously makes a payment and asserts that it fails for an invalid expiration month.
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
            Globalpay::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Invalid Expiry Date".to_string(),
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// This method makes a payment and then performs multiple refunds on the same payment if the payment was successful.
async fn should_refund_succeeded_payment_multiple_times() {
    CONNECTOR
        .make_payment_and_multiple_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            Globalpay::get_payment_info(),
        )
        .await;
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously checks if an authorized payment should be synced. This method first authorizes a payment using the CONNECTOR, then retrieves the transaction ID from the authorization response. It then uses the transaction ID to make a PSync request to the CONNECTOR and waits for the status to match the Authorized status. Once the status matches, it asserts that the response status is also Authorized.
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(None, Globalpay::get_payment_info())
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
            Globalpay::get_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures a payment and initiates a refund process for the captured payment. If the refund is successful, it asserts that the refund status is "Success".
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(None, None, None, Globalpay::get_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}
