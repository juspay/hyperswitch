use std::str::FromStr;

use api_models::payments::{Address, AddressDetails};
use cards::CardNumber;
use masking::Secret;
use router::{
    core::errors,
    types::{self, api, storage::enums, PaymentsAuthorizeData},
};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentInfo},
};

#[derive(Clone, Copy)]
struct PayeezyTest;
impl ConnectorActions for PayeezyTest {}
static CONNECTOR: PayeezyTest = PayeezyTest {};
impl utils::Connector for PayeezyTest {
        /// Retrieves connector data for the Payeezy connector, including the connector type, name, token retrieval method, and merchant connector ID.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Payeezy;
        types::api::ConnectorData {
            connector: Box::new(&Payeezy),
            // Remove `dummy_connector` feature gate from module in `main.rs` when updating this to use actual connector variant
            connector_name: types::Connector::DummyConnector1,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// Retrieves the authentication token for the connector.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .payeezy
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "payeezy".
    fn get_name(&self) -> String {
        "payeezy".to_string()
    }
}

impl PayeezyTest {
        /// Retrieves payment data for authorization, including the payment method data and authorization type.
    fn get_payment_data() -> Option<PaymentsAuthorizeData> {
        Some(PaymentsAuthorizeData {
            payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                card_number: CardNumber::from_str("4012000033330026").unwrap(),
                ..utils::CCardType::default().0
            }),
            ..utils::PaymentAuthorizeType::default().0
        })
    }

        /// Retrieves the payment information, if available.
    /// 
    /// Returns Some(PaymentInfo) if payment information is available, otherwise returns None.
    fn get_payment_info() -> Option<PaymentInfo> {
        Some(PaymentInfo {
            address: Some(types::PaymentAddress {
                billing: Some(Address {
                    address: Some(AddressDetails {
                        ..Default::default()
                    }),
                    phone: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
        /// Returns the interval for making requests, in milliseconds.
    fn get_request_interval(&self) -> u64 {
        20
    }
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment using the CONNECTOR. 
/// It expects a response with a status of Authorized and asserts that the response status is equal to enums::AttemptStatus::Authorized.
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(PayeezyTest::get_payment_data(), None)
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures an authorized payment using the Payeezy payment connector.
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(PayeezyTest::get_payment_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id =
        utils::get_connector_transaction_id(response.response.clone()).unwrap_or_default();
    let connector_meta = utils::get_connector_metadata(response.response);
    let capture_data = types::PaymentsCaptureData {
        connector_meta,
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
/// Asynchronously attempts to partially capture an authorized payment. 
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(PayeezyTest::get_payment_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id =
        utils::get_connector_transaction_id(response.response.clone()).unwrap_or_default();
    let connector_meta = utils::get_connector_metadata(response.response);
    let capture_data = types::PaymentsCaptureData {
        connector_meta,
        amount_to_capture: 50,
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
#[ignore]
/// This method is used to check if authorized payment should be synced.
async fn should_sync_authorized_payment() {
    // Method implementation goes here
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment, waits for a specified duration, and then voids the authorized payment.
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(PayeezyTest::get_payment_data(), None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let connector_payment_id =
        utils::get_connector_transaction_id(response.response.clone()).unwrap_or_default();
    let connector_meta = utils::get_connector_metadata(response.response);
    tokio::time::sleep(std::time::Duration::from_secs(
        CONNECTOR.get_request_interval(),
    ))
    .await; // to avoid 404 error
    let response = CONNECTOR
        .void_payment(
            connector_payment_id,
            Some(types::PaymentsCancelData {
                connector_meta,
                amount: Some(100),
                currency: Some(diesel_models::enums::Currency::USD),
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
/// This method is used to perform a refund for a manually captured payment. It first authorizes the payment, then captures it, and finally issues a refund for the captured payment. The method returns an error if any of the steps fail, and asserts that the refund status is 'Success'.
async fn should_refund_manually_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            PayeezyTest::get_payment_data(),
            PayeezyTest::get_payment_info(),
        )
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
            PayeezyTest::get_payment_info(),
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
                connector_metadata: refund_connector_metadata,
                ..utils::PaymentRefundType::default().0
            }),
            PayeezyTest::get_payment_info(),
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
/// Asynchronously refunds a manually captured payment by authorizing the payment, capturing the payment, and then refunding the captured amount. 
async fn should_partially_refund_manually_captured_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(
            PayeezyTest::get_payment_data(),
            PayeezyTest::get_payment_info(),
        )
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
            PayeezyTest::get_payment_info(),
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
                refund_amount: 50,
                connector_transaction_id: capture_txn_id,
                connector_metadata: refund_connector_metadata,
                ..utils::PaymentRefundType::default().0
            }),
            PayeezyTest::get_payment_info(),
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
#[ignore]
/// Asynchronously performs a manual synchronization for captured refunds. 
/// This method is responsible for handling the process of manually synchronizing captured refunds that were not automatically synced. 
async fn should_sync_manually_captured_refund() {
    // method implementation goes here
}

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment using the Payeezy connector. 
/// It first retrieves the payment data and payment information, then awaits the response from the connector to make the payment. 
/// It then asserts that the authorize response status is charged.
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            PayeezyTest::get_payment_data(),
            PayeezyTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore]
/// This method is responsible for determining whether an auto-captured payment should be synchronized. It will handle the logic for checking if the auto-captured payment needs to be synced with the payment gateway or not. 
async fn should_sync_auto_captured_payment() {
    // method implementation
    // ...
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously refunds a payment that was auto-captured by the connector.
async fn should_refund_auto_captured_payment() {
    let captured_response = CONNECTOR.make_payment(None, None).await.unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_meta = utils::get_connector_metadata(captured_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id.clone().unwrap(),
            Some(types::RefundsData {
                refund_amount: 100,
                connector_transaction_id: txn_id.unwrap(),
                connector_metadata: connector_meta,
                ..utils::PaymentRefundType::default().0
            }),
            PayeezyTest::get_payment_info(),
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
/// Asynchronously makes a payment using the CONNECTOR, captures the response, 
/// checks if the payment status is 'Charged', retrieves the transaction ID and 
/// metadata from the response, refunds a portion of the payment, and asserts 
/// that the refund status is 'Success'.
async fn should_partially_refund_succeeded_payment() {
    let captured_response = CONNECTOR.make_payment(None, None).await.unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_meta = utils::get_connector_metadata(captured_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id.clone().unwrap(),
            Some(types::RefundsData {
                refund_amount: 50,
                connector_transaction_id: txn_id.unwrap(),
                connector_metadata: connector_meta,
                ..utils::PaymentRefundType::default().0
            }),
            PayeezyTest::get_payment_info(),
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
/// Asynchronously makes a payment using the CONNECTOR, captures the response, and then attempts to refund the payment multiple times. 
///
async fn should_refund_succeeded_payment_multiple_times() {
    let captured_response = CONNECTOR.make_payment(None, None).await.unwrap();
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
                    ..utils::PaymentRefundType::default().0
                }),
                PayeezyTest::get_payment_info(),
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
#[ignore]
/// Asynchronously determines whether a refund should be synced with the payment processor.
async fn should_sync_refund() {
    // Method implementation goes here
}

// Cards Negative scenerios
// Creates a payment with incorrect card issuer.

#[actix_web::test]
/// Asynchronously performs a payment authorization using the `CONNECTOR` and verifies that it throws a `NotImplemented` error for unsupported issuers.
async fn should_throw_not_implemented_for_unsupported_issuer() {
    let authorize_data = Some(PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: CardNumber::from_str("630495060000000000").unwrap(),
            ..utils::CCardType::default().0
        }),
        capture_method: Some(enums::CaptureMethod::Automatic),
        ..utils::PaymentAuthorizeType::default().0
    });
    let response = CONNECTOR
        .make_payment(authorize_data, PayeezyTest::get_payment_info())
        .await;
    assert_eq!(
        *response.unwrap_err().current_context(),
        errors::ConnectorError::NotSupported {
            message: "card".to_string(),
            connector: "Payeezy",
        }
    )
}

// Creates a payment with incorrect CVC.
#[actix_web::test]
/// Asynchronously checks if the payment fails for an incorrect CVC.
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_cvc: Secret::new("12345d".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        *response.response.unwrap_err().message,
        "The cvv provided must be numeric".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// Asynchronously makes a payment using the CONNECTOR, with the intention of the payment failing due to an invalid expiration month on the card. 
///
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
        *response.response.unwrap_err().message,
        "Bad Request (25) - Invalid Expiry Date".to_string(),
    );
}
// Creates a payment with incorrect expiry year.
#[actix_web::test]
/// This method performs a payment using the CONNECTOR, but with incorrect expiry year, and checks that the payment fails with an error message "Expiry Date is invalid".
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
        "Expiry Date is invalid".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore]
/// Asynchronously attempts to fail void payment for auto-capture.
async fn should_fail_void_payment_for_auto_capture() {
    // method implementation goes here
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// Asynchronously attempts to capture a payment using the CONNECTOR service and expects the capture to fail with an invalid payment error.
async fn should_fail_capture_for_invalid_payment() {
    let connector_payment_id = "12345678".to_string();
    let capture_response = CONNECTOR
        .capture_payment(
            connector_payment_id,
            Some(types::PaymentsCaptureData {
                connector_meta: Some(
                    serde_json::json!({"transaction_tag" : "10069306640".to_string()}),
                ),
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("Bad Request (69) - Invalid Transaction Tag")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
/// Asynchronously makes a payment, captures the response, and attempts to refund an amount higher than the payment amount. 
/// The method expects the payment to be successful and the refund to fail with an "Invalid Refund" error message.
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let captured_response = CONNECTOR.make_payment(None, None).await.unwrap();
    assert_eq!(captured_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(captured_response.response.clone());
    let connector_meta = utils::get_connector_metadata(captured_response.response);
    let response = CONNECTOR
        .refund_payment(
            txn_id.clone().unwrap(),
            Some(types::RefundsData {
                refund_amount: 1500,
                connector_transaction_id: txn_id.unwrap(),
                connector_metadata: connector_meta,
                ..utils::PaymentRefundType::default().0
            }),
            PayeezyTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        String::from("Bad Request (64) - Invalid Refund"),
    );
}
