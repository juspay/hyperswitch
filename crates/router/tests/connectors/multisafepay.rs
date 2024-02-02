use api_models::payments::{Address, AddressDetails};
use masking::Secret;
use router::types::{self, api, storage::enums, PaymentAddress};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentInfo},
};

#[derive(Clone, Copy)]
struct MultisafepayTest;
impl ConnectorActions for MultisafepayTest {}
impl utils::Connector for MultisafepayTest {
        /// This method returns the data required for API connection with the Multisafepay connector, including the connector itself, the connector name, the token type, and the merchant connector ID.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Multisafepay;
        types::api::ConnectorData {
            connector: Box::new(&Multisafepay),
            connector_name: types::Connector::Multisafepay,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// Retrieves the authentication token for the connector. 
    /// 
    /// This method retrieves the authentication token for the connector by creating a new ConnectorAuthentication instance and converting it to the appropriate ConnectorAuthType using the utils::to_connector_auth_type function.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .multisafepay
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "multisafepay".
    fn get_name(&self) -> String {
        "multisafepay".to_string()
    }
}

static CONNECTOR: MultisafepayTest = MultisafepayTest {};

/// Retrieves the default payment information, including the default billing address with secret details for privacy.
fn get_default_payment_info() -> Option<PaymentInfo> {
    let address = Some(PaymentAddress {
        shipping: None,
        billing: Some(Address {
            address: Some(AddressDetails {
                first_name: Some(Secret::new("John".to_string())),
                last_name: Some(Secret::new("Doe".to_string())),
                line1: Some(Secret::new("Kraanspoor".to_string())),
                line2: Some(Secret::new("line2".to_string())),
                line3: Some(Secret::new("line3".to_string())),
                city: Some("Amsterdam".to_string()),
                zip: Some(Secret::new("1033SC".to_string())),
                country: Some(api_models::enums::CountryAlpha2::NL),
                state: Some(Secret::new("Amsterdam".to_string())),
            }),
            phone: None,
        }),
    });
    Some(PaymentInfo {
        address,
        ..utils::PaymentInfo::default()
    })
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// Asynchronously authorizes a payment and asserts that the response status is 'Authorized'.
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(None, get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// Asynchronously attempts to capture an authorized payment. 
/// 
/// This method uses the CONNECTOR to authorize and capture a payment with default payment information. 
/// It awaits the response and expects a successful capture payment response, asserting that the status is "Charged".
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(None, None, get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// Asynchronously attempts to partially capture an authorized payment. It uses the CONNECTOR to authorize and capture the payment with the specified amount and default payment information. If successful, it expects the response to have a status of `Charged` and asserts for equality. 
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            None,
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
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// Asynchronously attempts to synchronize an authorized payment using the CONNECTOR.
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(None, get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(
        authorize_response.status,
        enums::AttemptStatus::AuthenticationPending,
    );
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
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// This method authorizes and voids a payment, using the CONNECTOR to perform the operation. It creates a payment cancellation data with the provided connector transaction ID and cancellation reason, and then calls the authorize_and_void_payment method of the CONNECTOR with the cancellation data and default payment information. It awaits the response and asserts that the status of the response is Voided.
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            None,
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
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// Asynchronously captures a payment and initiates a refund if necessary. This method
/// utilizes the configured CONNECTOR to capture the payment and then initiate a refund
/// based on the response. It asserts that the refund status is "Success" and returns
/// nothing if the refund is successful. If the refund fails, an error will be raised.
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(None, None, None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[ignore = "Connector supports only 3ds flow"]
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
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// Asynchronously captures a payment and initiates a refund, then waits and retries until the refund status matches the specified status. It then asserts that the refund status is successful.
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
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// Asynchronously makes a payment using the default payment information and checks if the authorize response status is 'Charged'.
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// This method is used to synchronize an auto-captured payment with the connector. It first makes a payment using default payment information and ensures the response status is Charged. It then retrieves the transaction ID from the authorize response and uses it to synchronize the payment status with the connector. It retries the synchronization until the status matches Charged and asserts that the response status is also Charged.
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(None, get_default_payment_info())
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
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// Asynchronously makes a payment and then attempts to refund it. If the refund is successful, it asserts that the refund status is "Success".
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(None, None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// Asynchronously makes a payment and refunds a portion of the payment if the refund is successful.
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
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
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// Asynchronously makes a payment and attempts to refund the payment multiple times. 
/// This method uses the CONNECTOR to make the payment and initiate the refunds. 
/// It does not return any value, as it is an asynchronous method using the `await` keyword.
async fn should_refund_succeeded_payment_multiple_times() {
    CONNECTOR
        .make_payment_and_multiple_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await;
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// Asynchronously makes a payment and refund using the CONNECTOR and checks if the refund status is successful.
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(None, None, get_default_payment_info())
        .await
        .unwrap();
    let response = CONNECTOR
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

// Cards Negative scenerios
// Creates a payment with incorrect CVC.
#[ignore = "Connector doesn't fail invalid cvv scenario"]
#[actix_web::test]
/// Asynchronously attempts to make a payment with an incorrect Card Verification Code (CVC),
/// and asserts that the response is an error.
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_cvc: Secret::new("123498765".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert!(response.response.is_err());
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// Asynchronously makes a payment using the CONNECTOR, with an invalid expiration month for the card, and asserts that the payment should fail.
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
    assert!(response.response.is_err());
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
/// Asynchronously makes a payment with an incorrect expiry year and asserts that the payment fails.
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
    assert!(response.response.is_err());
}

// Voids a payment using automatic capture flow (Non 3DS).
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// This method tests that voiding a payment for auto-capture fails as expected. It first makes a payment using the default payment info and checks that the status is charged. Then it retrieves the transaction ID from the authorize response and ensures it is not empty. Finally, it attempts to void the payment using the retrieved transaction ID and verifies that the response contains an error message indicating that the PaymentIntent cannot be canceled because it has a status of succeeded.
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, None)
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "You cannot cancel this PaymentIntent because it has a status of succeeded."
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// Asynchronously attempts to capture a payment with an invalid payment ID, expecting the operation to fail with a specific error message. 
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("Something went wrong")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[ignore = "Connector supports only 3ds flow"]
#[actix_web::test]
/// This asynchronous method tests if the refund amount is higher than the payment amount and expects the payment to fail. It makes a payment and refund request with a refund amount of 150, then checks if the response contains an error. 
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert!(response.response.is_err());
}
