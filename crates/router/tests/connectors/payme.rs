use std::str::FromStr;

use api_models::payments::{Address, AddressDetails, OrderDetailsWithAmount};
use common_utils::pii::Email;
use masking::Secret;
use router::types::{self, api, storage::enums, PaymentAddress};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentAuthorizeType},
};

#[derive(Clone, Copy)]
struct PaymeTest;
impl ConnectorActions for PaymeTest {}
impl utils::Connector for PaymeTest {
        /// Retrieves the connector data for the Payme connector.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Payme;
        types::api::ConnectorData {
            connector: Box::new(&Payme),
            connector_name: types::Connector::Payme,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// This method retrieves the authentication token for the connector. It expects the connector to have authentication configured and will panic if the authentication configuration is missing. The method returns the authentication token as a `ConnectorAuthType`.
    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .payme
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// Returns the name "payme" as a String.
    fn get_name(&self) -> String {
        "payme".to_string()
    }
}

static CONNECTOR: PaymeTest = PaymeTest {};

/// Returns the default payment information, wrapped in an `Option`.
fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        address: Some(PaymentAddress {
            shipping: None,
            billing: Some(Address {
                address: Some(AddressDetails {
                    city: None,
                    country: None,
                    line1: None,
                    line2: None,
                    line3: None,
                    zip: None,
                    state: None,
                    first_name: Some(Secret::new("John".to_string())),
                    last_name: Some(Secret::new("Doe".to_string())),
                }),
                phone: None,
            }),
        }),
        auth_type: None,
        access_token: None,
        connector_meta_data: None,
        return_url: None,
        connector_customer: None,
        payment_method_token: None,
        country: None,
        currency: None,
        payout_method_data: None,
    })
}

/// Returns payment method details including order details, return URL, webhook URL, email, payment method data, and amount.
fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        order_details: Some(vec![OrderDetailsWithAmount {
            product_name: "iphone 13".to_string(),
            quantity: 1,
            amount: 1000,
            product_img_link: None,
            requires_shipping: None,
            product_id: None,
            category: None,
            brand: None,
            product_type: None,
        }]),
        router_return_url: Some("https://hyperswitch.io".to_string()),
        webhook_url: Some("https://hyperswitch.io".to_string()),
        email: Some(Email::from_str("test@gmail.com").unwrap()),
        payment_method_data: types::api::PaymentMethodData::Card(api::Card {
            card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
            card_cvc: Secret::new("123".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_holder_name: Some(masking::Secret::new("John Doe".to_string())),
            ..utils::CCardType::default().0
        }),
        amount: 1000,
        ..PaymentAuthorizeType::default().0
    })
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously authorizes a payment using the CONNECTOR. It calls the authorize_payment method with the provided payment method details and default payment information, and then asserts that the response status is Authorized.
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously attempts to capture an authorized payment using the `CONNECTOR` and asserts that the response status is `enums::AttemptStatus::Charged`.
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(payment_method_details(), None, get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not support partial capture"]
/// Asynchronously attempts to partially capture an authorized payment. It uses the payment method details, captures 50 units of the payment, and then expects a capture payment response with a status of Charged.
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            payment_method_details(),
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
#[actix_web::test]
#[ignore = "Connector does not supports sync"]
/// Asynchronously authorizes a payment, retrieves the transaction ID from the authorization response, and then synchronously retries until the payment status matches the authorized status. 
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
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
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not supports void"]
/// Asynchronously authorizes and voids a payment with the given payment method details and default payment info. 
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            payment_method_details(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Void flow not supported by Payme connector".to_string()
    );
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures a payment and refunds it manually. 
/// 
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
            None,
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
// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously captures a payment and performs a partial refund on the captured amount.
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
            None,
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

// Synchronizes a refund using the manual capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not supports sync"]
/// Asynchronously captures a payment and processes a refund, then retries the refund synchronization until the refund status matches the success status.
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
            None,
            None,
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
/// Asynchronously makes a payment using the CONNECTOR instance, with the given payment method details and default payment information. It then asserts that the response status is 'Charged'.
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not supports sync"]
/// Asynchronously checks if the auto-captured payment should be synchronized. It first makes a payment using the connector, then asserts that the authorize response status is 'Charged'. It then retrieves the connector transaction ID and ensures that it is not empty. Finally, it retries synchronizing the payment status until it matches 'Charged' and asserts that the response status is also 'Charged'.
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
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
                capture_method: Some(enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment, then refunds the payment if it was auto-captured. 
/// This method uses the CONNECTOR to make a payment and refund using the provided payment method details 
/// and default payment information. It then asserts that the refund status is successful.
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(payment_method_details(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
/// Asynchronously makes a payment and attempts to refund it partially if the payment has succeeded.
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
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
/// Asynchronously makes a payment and attempts to refund it multiple times using the CONNECTOR instance. The payment method details are obtained from the payment_method_details function, and the refund amount is set to 100. The payment information is retrieved using the get_default_payment_info function. This method returns an awaitable Future.
async fn should_refund_succeeded_payment_multiple_times() {
    CONNECTOR
        .make_payment_and_multiple_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 100,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await;
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not supports sync"]
/// Asynchronously makes a payment and refund, then checks the status of the refund to ensure it was successful. 
/// If the refund sync flow is not supported by the Payme connector, an assertion error is thrown.
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(payment_method_details(), None, get_default_payment_info())
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
        response.response.unwrap_err().message,
        "Refund Sync flow not supported by Payme connector",
    );
}

// Cards Negative scenerios
// Creates a payment with incorrect CVC.
#[actix_web::test]
/// Asynchronously makes a payment with incorrect CVC and asserts that it fails with an internal server error.
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 100,
                currency: enums::Currency::ILS,
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_cvc: Secret::new("12345".to_string()),
                    ..utils::CCardType::default().0
                }),
                order_details: Some(vec![OrderDetailsWithAmount {
                    product_name: "iphone 13".to_string(),
                    quantity: 1,
                    amount: 100,
                    product_img_link: None,
                    requires_shipping: None,
                    product_id: None,
                    category: None,
                    brand: None,
                    product_type: None,
                }]),
                router_return_url: Some("https://hyperswitch.io".to_string()),
                webhook_url: Some("https://hyperswitch.io".to_string()),
                email: Some(Email::from_str("test@gmail.com").unwrap()),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "internal_server_error".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
/// This asynchronous method tests the behavior of making a payment with an invalid expiration month on the card. It constructs a payment request with invalid expiration month data, sends the request using the CONNECTOR, and then asserts that the response contains an error message indicating an internal server error.
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 100,
                currency: enums::Currency::ILS,
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_month: Secret::new("20".to_string()),
                    ..utils::CCardType::default().0
                }),
                order_details: Some(vec![OrderDetailsWithAmount {
                    product_name: "iphone 13".to_string(),
                    quantity: 1,
                    amount: 100,
                    product_img_link: None,
                    requires_shipping: None,
                    product_id: None,
                    category: None,
                    brand: None,
                    product_type: None,
                }]),
                router_return_url: Some("https://hyperswitch.io".to_string()),
                webhook_url: Some("https://hyperswitch.io".to_string()),
                email: Some(Email::from_str("test@gmail.com").unwrap()),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "internal_server_error".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
/// Asynchronously makes a payment with incorrect expiry year and asserts that the payment
/// should fail with an internal server error response.
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                amount: 100,
                currency: enums::Currency::ILS,
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_year: Secret::new("2012".to_string()),
                    ..utils::CCardType::default().0
                }),
                order_details: Some(vec![OrderDetailsWithAmount {
                    product_name: "iphone 13".to_string(),
                    quantity: 1,
                    amount: 100,
                    product_img_link: None,
                    requires_shipping: None,
                    product_id: None,
                    category: None,
                    brand: None,
                    product_type: None,
                }]),
                router_return_url: Some("https://hyperswitch.io".to_string()),
                webhook_url: Some("https://hyperswitch.io".to_string()),
                email: Some(Email::from_str("test@gmail.com").unwrap()),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "internal_server_error".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
#[ignore = "Connector does not supports void"]
/// This method tests that void payment for auto-capture is expected to fail. It first makes a payment using the default payment method details and payment info, then checks that the payment status is "Charged" and retrieves the transaction ID from the authorize response. Next, it attempts to void the payment using the retrieved transaction ID and default payment info, and asserts that the void operation returns an error message indicating that void flow is not supported by the Payme connector.
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "Void flow not supported by Payme connector"
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
/// Asynchronously tests if the capture payment method fails for an invalid payment by attempting to capture a payment with an invalid payment ID and asserting that the response contains an internal server error message.
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("internal_server_error")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
/// Asynchronously performs a payment and refund operation, and asserts that the response
/// should fail for a refund amount higher than the payment amount. It uses the
/// CONNECTOR to make the payment and refund, with the provided payment method details,
/// refund amount, and payment information. It then asserts that the response
/// contains an internal server error message.
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 1500,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "internal_server_error",
    );
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
