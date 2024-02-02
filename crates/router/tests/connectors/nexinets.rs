use std::str::FromStr;

use cards::CardNumber;
use masking::Secret;
use router::types::{self, api, storage::enums, PaymentsAuthorizeData};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

#[derive(Clone, Copy)]
struct NexinetsTest;
impl ConnectorActions for NexinetsTest {}
static CONNECTOR: NexinetsTest = NexinetsTest {};
impl utils::Connector for NexinetsTest {
        /// Retrieves the connector data for Nexinets, including the connector object, connector name, token type, and merchant connector ID if available.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Nexinets;
        types::api::ConnectorData {
            connector: Box::new(&Nexinets),
            connector_name: types::Connector::Nexinets,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// This method retrieves the authentication token for the connector. It constructs a new ConnectorAuthentication object and extracts the authentication type from its configuration, converting it to the specified ConnectorAuthType using the to_connector_auth_type utility function.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .nexinets
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "nexinets" as a String.
    fn get_name(&self) -> String {
        "nexinets".to_string()
    }
}

/// Retrieves payment method details for authorization, including the currency, payment method data (in this case, card details), and the return URL for the router. If successful, returns an instance of PaymentsAuthorizeData wrapped in Some, otherwise returns None.
fn payment_method_details() -> Option<PaymentsAuthorizeData> {
    Some(PaymentsAuthorizeData {
        currency: diesel_models::enums::Currency::EUR,
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: CardNumber::from_str("374111111111111").unwrap(),
            ..utils::CCardType::default().0
        }),
        router_return_url: Some("https://google.com".to_string()),
        ..utils::PaymentAuthorizeType::default().0
    })
}
// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment and asserts that the response status is 'Authorized'.
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment, captures the authorized payment, and asserts that the response status is authorized and charged.
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id = "".to_string();
    let connector_meta = utils::get_connector_metadata(response.response);
    let capture_data = types::PaymentsCaptureData {
        connector_meta,
        currency: diesel_models::enums::Currency::EUR,
        ..utils::PaymentCaptureType::default().0
    };
    let capture_response = CONNECTOR
        .capture_payment(connector_payment_id, Some(capture_data), None)
        .await
        .unwrap();
    assert_eq!(capture_response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment, then partially captures the authorized payment.
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id = "".to_string();
    let connector_meta = utils::get_connector_metadata(response.response);
    let capture_data = types::PaymentsCaptureData {
        connector_meta,
        amount_to_capture: 50,
        currency: diesel_models::enums::Currency::EUR,
        ..utils::PaymentCaptureType::default().0
    };
    let capture_response = CONNECTOR
        .capture_payment(connector_payment_id, Some(capture_data), None)
        .await
        .unwrap();
    assert_eq!(capture_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously initiates the process to synchronize an authorized payment with the payment gateway.
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let connector_meta = utils::get_connector_metadata(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                encoded_data: None,
                capture_method: None,
                sync_type: types::SyncRequestType::SinglePaymentSync,
                connector_meta,
                mandate_id: None,
            }),
            None,
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// This method is used to void an authorized payment. It first authorizes a payment method and then voids the payment with the specified details. 
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id = "".to_string();
    let connector_meta = utils::get_connector_metadata(response.response);
    let response = CONNECTOR
        .void_payment(
            connector_payment_id,
            Some(types::PaymentsCancelData {
                connector_meta,
                amount: Some(100),
                currency: Some(diesel_models::enums::Currency::EUR),
                ..utils::PaymentCancelType::default().0
            }),
            None,
        )
        .await
        .unwrap();

    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously refunds a manually captured payment by authorizing the payment, capturing the payment, and then refunding the captured payment.
async fn should_refund_manually_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let capture_connector_meta = utils::get_connector_metadata(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id,
            Some(types::PaymentsCaptureData {
                currency: diesel_models::enums::Currency::EUR,
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    let capture_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_metadata = utils::get_connector_metadata(capture_response.response);
    let response = CONNECTOR
        .refund_payment(
            capture_txn_id.clone(),
            Some(types::RefundsData {
                connector_transaction_id: capture_txn_id,
                currency: diesel_models::enums::Currency::EUR,
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
/// Asynchronously initiates a partial refund for a manually captured payment. It authorizes the payment, captures the payment, and then refunds a specified amount of the captured payment.
async fn should_partially_refund_manually_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let capture_connector_meta = utils::get_connector_metadata(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                currency: diesel_models::enums::Currency::EUR,
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    let capture_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_metadata = utils::get_connector_metadata(capture_response.response);
    let response = CONNECTOR
        .refund_payment(
            capture_txn_id.clone(),
            Some(types::RefundsData {
                refund_amount: 10,
                connector_transaction_id: capture_txn_id,
                currency: diesel_models::enums::Currency::EUR,
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

// Synchronizes a refund using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures a payment, refunds the captured amount, and retries the refund until a success status is achieved.
async fn should_sync_manually_captured_refund() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), None)
        .await
        .expect("Authorize payment response");
    let txn_id = "".to_string();
    let capture_connector_meta = utils::get_connector_metadata(authorize_response.response);
    let capture_response = CONNECTOR
        .capture_payment(
            txn_id.clone(),
            Some(types::PaymentsCaptureData {
                currency: diesel_models::enums::Currency::EUR,
                connector_meta: capture_connector_meta,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .expect("Capture payment response");
    let capture_txn_id =
        utils::get_connector_transaction_id(capture_response.response.clone()).unwrap();
    let refund_connector_metadata = utils::get_connector_metadata(capture_response.response);
    let refund_response = CONNECTOR
        .refund_payment(
            capture_txn_id.clone(),
            Some(types::RefundsData {
                refund_amount: 100,
                connector_transaction_id: capture_txn_id.clone(),
                currency: diesel_models::enums::Currency::EUR,
                connector_metadata: refund_connector_metadata.clone(),
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let transaction_id = Some(
        refund_response
            .response
            .clone()
            .unwrap()
            .connector_refund_id,
    );
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response
                .response
                .clone()
                .unwrap()
                .connector_refund_id,
            Some(types::RefundsData {
                connector_refund_id: transaction_id,
                connector_transaction_id: capture_txn_id,
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

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment using the CONNECTOR. This method calls the make_payment
/// function with the provided payment method details and awaits the response. It then asserts
/// that the status of the authorize_response is equal to enums::AttemptStatus::Charged.
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// This method is used to synchronize an auto-captured payment with the connector. It first makes a payment using the specified payment method details, then retrieves the transaction id and metadata from the response. After that, it retries the synchronization process until the status matches the specified attempt status. Finally, it asserts that the response status is charged.
async fn should_sync_auto_captured_payment() {
    let cap_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(cap_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(cap_response.response.clone()).unwrap();
    let connector_meta = utils::get_connector_metadata(cap_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(txn_id),
                capture_method: Some(enums::CaptureMethod::Automatic),
                connector_meta,
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously refunds a previously auto-captured payment made through the connector, 
/// ensuring that the payment status is 'Charged' before initiating the refund process.
async fn should_refund_auto_captured_payment() {
    let captured_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_metadata = utils::get_connector_metadata(captured_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id.clone().unwrap(),
            Some(types::RefundsData {
                refund_amount: 100,
                currency: diesel_models::enums::Currency::EUR,
                connector_transaction_id: txn_id.unwrap(),
                connector_metadata,
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

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Makes a partial refund for a succeeded payment by first making a payment using the CONNECTOR, 
/// then capturing the response and checking if the status is Charged. If the status is Charged, 
/// it then gets the transaction ID and metadata from the captured response. Finally, it makes a 
/// refund payment using the CONNECTOR with the specified refund amount, currency, transaction ID, 
/// metadata, and other default refund type parameters, and checks if the refund status is Success.
async fn should_partially_refund_succeeded_payment() {
    let captured_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_meta = utils::get_connector_metadata(captured_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id.clone().unwrap(),
            Some(types::RefundsData {
                refund_amount: 50,
                currency: diesel_models::enums::Currency::EUR,
                connector_transaction_id: txn_id.unwrap(),
                connector_metadata: connector_meta,
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

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to refund a successfully charged payment multiple times.
///
async fn should_refund_succeeded_payment_multiple_times() {
    let captured_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_meta = utils::get_connector_metadata(captured_response.response);
    for _x in 0..2 {
        let refund_response = CONNECTOR
            .refund_payment(
                txn_id.clone().unwrap(),
                Some(types::RefundsData {
                    connector_metadata: connector_meta.clone(),
                    connector_transaction_id: txn_id.clone().unwrap(),
                    refund_amount: 50,
                    currency: diesel_models::enums::Currency::EUR,
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
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a refund request and synchronously retries until the refund status matches the expected status. 
async fn should_sync_refund() {
    let captured_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_metadata = utils::get_connector_metadata(captured_response.response).clone();
    let refund_response = CONNECTOR
        .refund_payment(
            txn_id.clone().unwrap(),
            Some(types::RefundsData {
                connector_transaction_id: txn_id.clone().unwrap(),
                refund_amount: 100,
                currency: diesel_models::enums::Currency::EUR,
                connector_metadata: connector_metadata.clone(),
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let transaction_id = Some(
        refund_response
            .response
            .clone()
            .unwrap()
            .connector_refund_id,
    );
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response
                .response
                .clone()
                .unwrap()
                .connector_refund_id,
            Some(types::RefundsData {
                connector_refund_id: transaction_id,
                connector_transaction_id: txn_id.unwrap(),
                connector_metadata,
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

// Creates a payment with incorrect CVC.
#[actix_web::test]
/// Asynchronously makes a payment with incorrect CVC and expects it to fail with a specific error message.
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(PaymentsAuthorizeData {
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
        "payment.verification : Bad value for 'payment.verification'. Expected: string of length in range 3 <=> 4 representing a valid creditcard verification number.".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// This method tests if a payment fails for an invalid expiration month. It makes a payment with a card expiration month set to "20" and expects the response to contain an error message indicating that the expiration month should be a string of length 2 in the range '01' <=> '12' representing a valid credit card expiration date greater than or equal to the current date.
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(PaymentsAuthorizeData {
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
        "payment.expiryMonth : Bad value for 'payment.expiryMonth'. Expected: string of length 2 in range '01' <=> '12' representing the month in a valid creditcard expiry date >= current date.".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
/// Asynchronously makes a payment with incorrect expiry year and expects the payment to fail, asserting that the response contains the expected error message.
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(PaymentsAuthorizeData {
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
        "payment.expiryYear : Bad value for 'payment.expiryYear'. Expected: string of length 2 in range '01' <=> '99' representing the year in a valid creditcard expiry date >= current date.".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
/// This method attempts to make a void payment for an auto-captured payment. It first makes a payment using the provided payment method details, then attempts to void the payment using the captured transaction ID and additional payment cancellation data. It expects the void payment to fail with a specific error message, and asserts this to ensure the intended behavior.
async fn should_fail_void_payment_for_auto_capture() {
    let captured_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone()).unwrap();
    let connector_meta = utils::get_connector_metadata(captured_response.response);
    let void_response = CONNECTOR
        .void_payment(
            txn_id,
            Some(types::PaymentsCancelData {
                cancellation_reason: Some("requested_by_customer".to_string()),
                amount: Some(100),
                currency: Some(diesel_models::enums::Currency::EUR),
                connector_meta,
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "transactionId : Operation not allowed!"
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// Asynchronously attempts to capture a payment with invalid payment details and expects the capture to fail with a specific error message. 
async fn should_fail_capture_for_invalid_payment() {
    let connector_payment_id = "".to_string();
    let capture_response = CONNECTOR
        .capture_payment(
            connector_payment_id,
            Some(types::PaymentsCaptureData {
                connector_meta: Some(
                    serde_json::json!({"transaction_id" : "transaction_usmh41hymb",
                        "order_id" : "tjil1ymxsz",
                        "psync_flow" : "PREAUTH"
                    }),
                ),
                amount_to_capture: 50,
                currency: diesel_models::enums::Currency::EUR,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("transactionId : Transaction does not belong to order.")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
/// This method tests that refunding a payment with an amount higher than the initial payment amount will fail and return an error with the correct message.
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let captured_response = CONNECTOR
        .make_payment(payment_method_details(), None)
        .await
        .unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_meta = utils::get_connector_metadata(captured_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id.clone().unwrap(),
            Some(types::RefundsData {
                refund_amount: 150,
                currency: diesel_models::enums::Currency::EUR,
                connector_transaction_id: txn_id.unwrap(),
                connector_metadata: connector_meta,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "initialAmount : Bad value for 'initialAmount'. Expected: Positive integer between 1 and maximum available amount (debit/capture.initialAmount - debit/capture.refundedAmount.",
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
