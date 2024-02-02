use std::str::FromStr;

use api_models::payments::PaymentMethodData;
use masking::Secret;
use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct BamboraTest;
impl ConnectorActions for BamboraTest {}
impl utils::Connector for BamboraTest {
        /// Retrieves the connector data for Bambora, including the connector type, connector name,
    /// token retrieval method, and merchant connector ID if available.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Bambora;
        types::api::ConnectorData {
            connector: Box::new(&Bambora),
            connector_name: types::Connector::Bambora,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// This method returns the authentication token for the connector. It retrieves the authentication configuration for the Bambora connector, converts it to the appropriate type, and returns the authentication token.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .bambora
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "bambora" as a String.
    fn get_name(&self) -> String {
        "bambora".to_string()
    }
}

static CONNECTOR: BamboraTest = BamboraTest {};

/// Returns the default payment authorization data for a card payment method, including the card number, expiration year, and CVC. This method constructs a `PaymentsAuthorizeData` object with default values for the payment method data and the payment authorize type.
fn get_default_payment_authorize_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: cards::CardNumber::from_str("4030000010001234").unwrap(),
            card_exp_year: Secret::new("25".to_string()),
            card_cvc: Secret::new("123".to_string()),
            ..utils::CCardType::default().0
        }),
        ..utils::PaymentAuthorizeType::default().0
    })
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment using the default payment authorize data and asserts that the response status is "Authorized".
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(get_default_payment_authorize_data(), None)
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to capture an authorized payment. It calls the `authorize_and_capture_payment` method of the `CONNECTOR` with default payment authorization data, and awaits the response. It then asserts that the response status is `enums::AttemptStatus::Charged`.
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(get_default_payment_authorize_data(), None, None)
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes and partially captures a payment for a specified amount using the CONNECTOR. The method uses the default payment authorize data and captures 50 units of the payment, expecting a response with a status of 'Charged'. If successful, it returns the capture payment response. If unsuccessful, it will panic with a message indicating the failure of the capture payment response.
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            get_default_payment_authorize_data(),
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
/// Asynchronously attempts to synchronize an authorized payment by authorizing the payment with the CONNECTOR, obtaining the transaction ID from the authorization response, and then retrying synchronization until the status matches the authorized status. If successful, it asserts that the response status is authorized. 
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(get_default_payment_authorize_data(), None)
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                mandate_id: None,
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                encoded_data: None,
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                sync_type: types::SyncRequestType::SinglePaymentSync,
                connector_meta: None,
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously voids an authorized payment by sending a request to the connector. 
/// If the authorization and voiding of the payment is successful, it asserts that the response status is 'Voided'.
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            get_default_payment_authorize_data(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
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
/// Asynchronously captures a payment and initiates a refund process for the captured payment. 
/// If successful, the refund status is checked to ensure that the refund was successful.
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(get_default_payment_authorize_data(), None, None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures a payment and partially refunds it using the CONNECTOR.
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            get_default_payment_authorize_data(),
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

// Synchronizes a refund using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures a payment and refunds it, then checks the refund status
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(get_default_payment_authorize_data(), None, None, None)
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
#[actix_web::test]
/// Asynchronously checks if a payment should be made by calling the `make_payment` method of the `CONNECTOR` with default payment authorization data and no additional options.
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_default_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously sends a payment authorize request, checks the authorize response status, retrieves the transaction ID, and then makes a retry till status matches request for automatic payment capture. 
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_default_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
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
                connector_meta: None,
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment and then attempts to refund the payment if it was auto-captured. 
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(get_default_payment_authorize_data(), None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a partial refund for a succeeded payment. It makes a payment and then performs a refund for the specified refund amount. It then asserts that the refund status is a success.
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            get_default_payment_authorize_data(),
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

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously checks if a refund should be synchronized with the payment system.
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(get_default_payment_authorize_data(), None, None)
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
// Creates a payment with incorrect card number.
#[actix_web::test]
/// Asynchronously makes a payment using an incorrect card number and asserts that the payment fails with an "Invalid Card Number" error message.
async fn should_fail_payment_for_incorrect_card_number() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("1234567891011").unwrap(),
                    card_exp_year: Secret::new("25".to_string()),
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
        "Invalid Card Number".to_string(),
    );
}

// Creates a payment with incorrect CVC.
#[actix_web::test]
/// Asynchronously makes a payment and asserts that the payment fails for an incorrect Card Verification Code (CVC).
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: PaymentMethodData::Card(api::Card {
                    card_exp_year: Secret::new("25".to_string()),
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
        response.response.unwrap_err().reason,
        Some(r#"[{"field":"card:cvd","message":"Invalid card CVD"},{"field":"card:cvd","message":"Invalid card CVD"}]"#.to_string())
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// This method tests that a payment fails when using an invalid expiration month for a card.
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: PaymentMethodData::Card(api::Card {
                    card_exp_month: Secret::new("20".to_string()),
                    card_number: cards::CardNumber::from_str("4030000010001234").unwrap(),
                    card_exp_year: Secret::new("25".to_string()),
                    card_cvc: Secret::new("123".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().reason,
        Some(r#"[{"field":"card:expiry_month","message":"Invalid expiry date"}]"#.to_string())
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
/// Checks if the payment fails for an incorrect expiry year of the card.
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: PaymentMethodData::Card(api::Card {
                    card_exp_year: Secret::new("2000".to_string()),
                    card_number: cards::CardNumber::from_str("4030000010001234").unwrap(),
                    card_cvc: Secret::new("123".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().reason,
        Some(r#"[{"field":"card:expiry_year","message":"Invalid expiration year"}]"#.to_string())
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
/// This method is used to test the scenario where a void payment for an auto-captured transaction should fail. It first makes a payment using default payment authorization data and checks if the status is charged. Then it retrieves the transaction id and ensures it is not empty. After that, it attempts to void the payment using the obtained transaction id and expects the void operation to fail with the specific error message "Transaction cannot be adjusted".
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(get_default_payment_authorize_data(), None)
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
        "Transaction cannot be adjusted"
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// Asynchronously attempts to capture a payment and expects it to fail due to invalid payment information.
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("Missing or invalid payment information - Please validate all required payment information.")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
/// Asynchronously makes a payment and refund where the refund amount is higher than the payment amount. 
/// It then asserts that the refund status is success.
async fn should_succeed_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            get_default_payment_authorize_data(),
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
        enums::RefundStatus::Success
    );
}
