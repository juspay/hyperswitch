use std::{str::FromStr, time::Duration};

use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

struct NmiTest;
impl ConnectorActions for NmiTest {}
impl utils::Connector for NmiTest {
        /// Retrieves the connector data for the current instance. This method returns a `ConnectorData` struct that contains the information about the connector, connector name, token type, and merchant connector ID.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Nmi;
        types::api::ConnectorData {
            connector: Box::new(&Nmi),
            connector_name: types::Connector::Nmi,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// Retrieves the authentication token for the connector. 
    /// 
    /// This method returns the authentication token for the connector by first creating a new instance of `ConnectorAuthentication` and accessing the `nmi` authentication type. If the `nmi` authentication type is not available, it will panic with the message "Missing connector authentication configuration". It then converts the authentication type into the appropriate `ConnectorAuthType` using the `to_connector_auth_type` function from the `utils` module and returns it.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .nmi
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Retrieves the name of the object.
    fn get_name(&self) -> String {
        "nmi".to_string()
    }
}

static CONNECTOR: NmiTest = NmiTest {};

/// Retrieves payment authorization data containing payment method data with a card number and an amount.
fn get_payment_authorize_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
            ..utils::CCardType::default().0
        }),
        amount: 2023,
        ..utils::PaymentAuthorizeType::default().0
    })
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment and ensures that only authorized payments are processed. 
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .expect("Authorize payment response");
    let transaction_id = utils::get_connector_transaction_id(response.response).unwrap();
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    // Assert the sync response, it will be authorized in case of manual capture, for automatic it will be Completed Success
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures an authorized payment using the CONNECTOR. It first authorizes the payment, then captures it, and finally checks the status of the capture process.
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorizing);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    let capture_response = CONNECTOR
        .capture_payment(transaction_id.clone(), None, None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.status,
        enums::AttemptStatus::CaptureInitiated
    );
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously performs the steps for partially capturing an authorized payment, including authorizing the payment, capturing a portion of the payment, and checking the status of the capture operation.
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorizing);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    let capture_response = CONNECTOR
        .capture_payment(
            transaction_id.clone(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 1000,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        capture_response.status,
        enums::AttemptStatus::CaptureInitiated
    );

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// This method is used to initiate and complete a void process for an authorized payment.
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorizing);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);

    let void_response = CONNECTOR
        .void_payment(
            transaction_id.clone(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("user_cancel".to_string()),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(void_response.status, enums::AttemptStatus::VoidInitiated);
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Voided,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// This method is responsible for refunding a manually captured payment. It first authorizes a payment, then captures the payment manually, initiates a refund, and finally checks the status of the refund until it reaches a pending state.
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorizing);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    let capture_response = CONNECTOR
        .capture_payment(transaction_id.clone(), None, None)
        .await
        .unwrap();
    assert_eq!(
        capture_response.status,
        enums::AttemptStatus::CaptureInitiated
    );
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);

    let refund_response = CONNECTOR
        .refund_payment(transaction_id.clone(), None, None)
        .await
        .unwrap();
    assert_eq!(refund_response.status, enums::AttemptStatus::Pending);
    let sync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Pending,
            refund_response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        sync_response.response.unwrap().refund_status,
        enums::RefundStatus::Pending
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Should partially refund a manually captured payment by authorizing the payment, capturing a portion of the amount, initiating a refund for the captured amount, and checking the status of the refund until it is pending.
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorizing);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    let capture_response = CONNECTOR
        .capture_payment(
            transaction_id.clone(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 2023,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        capture_response.status,
        enums::AttemptStatus::CaptureInitiated
    );
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);

    let refund_response = CONNECTOR
        .refund_payment(
            transaction_id.clone(),
            Some(types::RefundsData {
                refund_amount: 1023,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(refund_response.status, enums::AttemptStatus::Pending);
    let sync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Pending,
            refund_response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        sync_response.response.unwrap().refund_status,
        enums::RefundStatus::Pending
    );
}

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment using the CONNECTOR, then checks and asserts the status of the payment to ensure it is pending.
async fn should_make_payment() {
    let response = CONNECTOR
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously refunds an automatically captured payment by making a payment, initiating a refund, and checking the refund status until it is pending.
async fn should_refund_auto_captured_payment() {
    // Make a payment and capture the transaction ID
    let response = CONNECTOR
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    // Check the payment status until it is pending
    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);

    // Initiate a refund and check the refund status until it is pending
    let refund_response = CONNECTOR
        .refund_payment(transaction_id.clone(), None, None)
        .await
        .unwrap();
    assert_eq!(refund_response.status, enums::AttemptStatus::Pending);
    let sync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Pending,
            refund_response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        sync_response.response.unwrap().refund_status,
        enums::RefundStatus::Pending
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// This asynchronous method is used to partially refund a succeeded payment. It first makes a payment authorization request, then captures the payment, and finally initiates a refund for a specified refund amount. It also ensures that the refund status is set to Pending. 
async fn should_partially_refund_succeeded_payment() {
    let response = CONNECTOR
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);

    let refund_response = CONNECTOR
        .refund_payment(
            transaction_id.clone(),
            Some(types::RefundsData {
                refund_amount: 1000,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(refund_response.status, enums::AttemptStatus::Pending);
    let sync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Pending,
            refund_response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        sync_response.response.unwrap().refund_status,
        enums::RefundStatus::Pending
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// This method is used to test the refund process for a successful payment multiple times. It first makes a payment and ensures that the status is CaptureInitiated. Then it retries syncing the payment until the status is Pending. After that, it attempts to refund the payment twice and verifies that the refund status is Pending each time.
async fn should_refund_succeeded_payment_multiple_times() {
    let response = CONNECTOR
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);

    //try refund for previous payment
    let transaction_id = utils::get_connector_transaction_id(response.response).unwrap();
    for _x in 0..2 {
        tokio::time::sleep(Duration::from_secs(5)).await; // to avoid 404 error
        let refund_response = CONNECTOR
            .refund_payment(
                transaction_id.clone(),
                Some(types::RefundsData {
                    refund_amount: 50,
                    ..utils::PaymentRefundType::default().0
                }),
                None,
            )
            .await
            .unwrap();
        let sync_response = CONNECTOR
            .rsync_retry_till_status_matches(
                enums::RefundStatus::Pending,
                refund_response.response.unwrap().connector_refund_id,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            sync_response.response.unwrap().refund_status,
            enums::RefundStatus::Pending,
        );
    }
}

// Creates a payment with incorrect CVC.
#[ignore = "Connector returns SUCCESS status in case of invalid CVC"]
#[actix_web::test]
/// This method is used to simulate a payment failure when an incorrect CVC (Card Verification Code) is provided during the payment process. 
async fn should_fail_payment_for_incorrect_cvc() {
    // implementation goes here
}

// Creates a payment with incorrect expiry month.
#[ignore = "Connector returns SUCCESS status in case of expired month."]
#[actix_web::test]
/// Asynchronously checks if the payment should fail for an invalid expiration month.
async fn should_fail_payment_for_invalid_exp_month() {}

// Creates a payment with incorrect expiry year.
#[ignore = "Connector returns SUCCESS status in case of expired year."]
#[actix_web::test]
/// Asynchronously checks if a payment should fail for an incorrect expiry year.
async fn should_fail_payment_for_incorrect_expiry_year() {
    // implementation goes here
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
/// This method tests that voiding a payment with automatic capture method fails, by first making a payment with automatic capture, then attempting to void the payment, and asserting that the void operation fails.
async fn should_fail_void_payment_for_auto_capture() {
    let response = CONNECTOR
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);

    let void_response = CONNECTOR
        .void_payment(transaction_id.clone(), None, None)
        .await
        .unwrap();
    assert_eq!(void_response.status, enums::AttemptStatus::VoidFailed);
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// This method attempts to authorize a payment, then waits for the authorization to be successful before capturing the payment. It then asserts that the capture fails for an invalid payment.
async fn should_fail_capture_for_invalid_payment() {
    let response = CONNECTOR
        .authorize_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorizing);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Manual),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    let capture_response = CONNECTOR
        .capture_payment("7899353591".to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(capture_response.status, enums::AttemptStatus::CaptureFailed);
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
/// This async function tests that a refund request fails if the refund amount is higher than the payment amount. It first makes a payment, captures the transaction ID, and then attempts to refund a higher amount than the payment. It expects the refund status to be a failure.
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment(get_payment_authorize_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::CaptureInitiated);
    let transaction_id = utils::get_connector_transaction_id(response.response.to_owned()).unwrap();

    let sync_response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Pending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                capture_method: Some(types::storage::enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status, enums::AttemptStatus::Pending);
    let refund_response = CONNECTOR
        .refund_payment(
            transaction_id,
            Some(types::RefundsData {
                refund_amount: 3024,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Failure
    );
}
