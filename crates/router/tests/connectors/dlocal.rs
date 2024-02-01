use std::str::FromStr;

use api_models::payments::Address;
use masking::Secret;
use router::types::{self, api, storage::enums, PaymentAddress};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentInfo},
};

#[derive(Clone, Copy)]
struct DlocalTest;
impl ConnectorActions for DlocalTest {}
impl utils::Connector for DlocalTest {
        /// This method returns the ConnectorData with the Dlocal connector information.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Dlocal;
        types::api::ConnectorData {
            connector: Box::new(&Dlocal),
            connector_name: types::Connector::Dlocal,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// This method retrieves the authentication token for the connector. It first creates a new ConnectorAuthentication object and then expects the dlocal authentication configuration to be present. It then converts the authentication type to the appropriate ConnectorAuthType using the 'to_connector_auth_type' utility function.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .dlocal
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Retrieves the name of the object.
    /// 
    /// # Returns
    /// 
    /// A `String` containing the name "dlocal".
    fn get_name(&self) -> String {
        "dlocal".to_string()
    }
}

static CONNECTOR: DlocalTest = DlocalTest {};

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// This asynchronous method is responsible for authorizing a payment using the CONNECTOR. It sends a request to authorize a payment with the payment information retrieved from the get_payment_info() method. It then expects a response and checks if the status of the response is 'Authorized'. If the status is 'Authorized', the method passes, otherwise it will fail.
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(None, Some(get_payment_info()))
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to capture an authorized payment. 
///
/// This method uses the CONNECTOR to authorize and capture a payment using the provided payment information. 
/// It then awaits the response and expects a successful capture payment response. 
/// If the capture is successful, it asserts that the response status is 'Charged'.
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(None, None, Some(get_payment_info()))
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to partially capture an authorized payment. 
/// If successful, it captures an amount of 50 units and expects the payment status to be 'Charged'.
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            None,
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            Some(get_payment_info()),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to sync an authorized payment by authorizing the payment,
/// retrieving the transaction ID, and then retrying the payment sync until the status matches
/// the authorized status. It then prints and asserts the status of the response.
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(None, Some(get_payment_info()))
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
            Some(get_payment_info()),
        )
        .await
        .expect("PSync response");
    println!("{}", response.status);
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes and voids a payment, with the option to provide cancellation data and payment information. 
/// 
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            None,
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            Some(get_payment_info()),
        )
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures a payment and initiates a refund if necessary. 
/// It checks the payment information and captures the payment. If the capture is successful, it then initiates a refund and checks the refund status. 
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(None, None, None, Some(get_payment_info()))
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to partially refund a manually captured payment. 
///
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            None,
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            Some(get_payment_info()),
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
/// Asynchronously captures a payment and initiates a refund, then synchronously retries until the refund status matches 'Success' and asserts that the refund status is 'Success'.
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(None, None, None, Some(get_payment_info()))
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            Some(get_payment_info()),
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
/// Asynchronously makes a payment using the CONNECTOR and retrieves the authorization response. 
/// The method uses the payment information obtained from the get_payment_info() function and ensures 
/// that the payment is successfully charged before proceeding.
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(None, Some(get_payment_info()))
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously checks if an auto-captured payment should be synced. 
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(None, Some(get_payment_info()))
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
            Some(get_payment_info()),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment and then attempts to refund it. If the refund is successful, it asserts that the refund status is "Success".
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(None, None, Some(get_payment_info()))
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a partial refund for a succeeded payment. It calls the `make_payment_and_refund` method of the `CONNECTOR` struct with the refund amount of 50 and the payment information obtained from the `get_payment_info` function. It then asserts that the refund response is successful.
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            Some(get_payment_info()),
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
/// Makes a payment and performs multiple refunds on it. This method uses the CONNECTOR to make a payment, then initiates multiple refunds on the payment with a refund amount of 50. It also includes the payment information obtained from get_payment_info(). This method is asynchronous and awaits the completion of the payment and refunds.
async fn should_refund_succeeded_payment_multiple_times() {
    CONNECTOR
        .make_payment_and_multiple_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            Some(get_payment_info()),
        )
        .await;
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously performs a refund and then syncs the refund status until it matches the expected success status.
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(None, None, Some(get_payment_info()))
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            Some(get_payment_info()),
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
/// Asynchronously makes a payment using an incorrect card number and expects the payment to fail with a specific error message and reason.
async fn should_fail_payment_for_incorrect_card_number() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("1891011").unwrap(),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            Some(get_payment_info()),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "Invalid parameter",);
    assert_eq!(x.reason, Some("card.number".to_string()));
}

// Creates a payment with incorrect CVC.
#[actix_web::test]
/// Asynchronously makes a payment using the CONNECTOR, with the intention of failing the payment due to an incorrect card verification code (CVC). It constructs a payment request with a card CVC of "1ad2345" and verifies that the response contains an error message of "Invalid parameter" and a reason of "card.cvv".
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_cvc: Secret::new("1ad2345".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            Some(get_payment_info()),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "Invalid parameter",);
    assert_eq!(x.reason, Some("card.cvv".to_string()));
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// Asynchronously tests that a payment should fail for an invalid expiration month on a card.
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_month: Secret::new("201".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            Some(get_payment_info()),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "Invalid parameter",);
    assert_eq!(x.reason, Some("card.expiration_month".to_string()));
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
/// Asynchronously tests if a payment should fail for incorrect expiry year of the card.
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_year: Secret::new("20001".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            Some(get_payment_info()),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, "Invalid parameter",);
    assert_eq!(x.reason, Some("card.expiration_year".to_string()));
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
/// This method is used to test the void payment functionality for auto-capture transactions. It first makes a payment request to the CONNECTOR, then checks if the payment was successfully captured. If it was, it retrieves the transaction ID and uses it to void the payment. The method then asserts that the void payment response contains an error with the code "5021" and the message "Acquirer could not process the request".
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(None, Some(get_payment_info()))
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, None)
        .await
        .unwrap();
    let x = void_response.response.unwrap_err();
    assert_eq!(x.code, "5021",);
    assert_eq!(x.message, "Acquirer could not process the request");
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// Asynchronously attempts to capture a payment with an invalid payment ID and expects the capture to fail with a specific error code.
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456sdf789".to_string(), None, Some(get_payment_info()))
        .await
        .unwrap();
    let x = capture_response.response.unwrap_err();
    assert_eq!(x.code, "3003",);
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
/// This asynchronous method tests whether the refund amount is higher than the payment amount by making a payment and refund request to the connector. It then checks for the expected error response code and message to ensure that the refund amount exceeding the payment amount results in the correct error being returned.
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            Some(get_payment_info()),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    println!("response from refund amount higher payment");
    println!("{}", x.code);
    assert_eq!(x.code, "5007",);
    assert_eq!(x.message, "Amount exceeded",);
}

/// Retrieves the payment information including the billing address and country details.
pub fn get_payment_info() -> PaymentInfo {
    PaymentInfo {
        address: Some(PaymentAddress {
            shipping: None,
            billing: Some(Address {
                phone: None,
                address: Some(api::AddressDetails {
                    city: None,
                    country: Some(api_models::enums::CountryAlpha2::PA),
                    line1: None,
                    line2: None,
                    line3: None,
                    zip: None,
                    state: None,
                    first_name: None,
                    last_name: None,
                }),
            }),
        }),
        auth_type: None,
        access_token: None,
        connector_meta_data: None,
        ..Default::default()
    }
}
// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
