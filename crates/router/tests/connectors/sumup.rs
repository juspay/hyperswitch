use masking::Secret;
use router::{
    types::{self, api, storage::enums,
}};

use crate::utils::{self, ConnectorActions};
use test_utils::connector_auth;

#[derive(Clone, Copy)]
struct SumupTest;
impl ConnectorActions for SumupTest {}
impl utils::Connector for SumupTest {
    fn get_data(&self) -> api::ConnectorData {
        use router::connector::Sumup;
        api::ConnectorData {
            connector: Box::new(Sumup::new()),
            connector_name: types::Connector::Sumup,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .sumup
                .expect("Missing connector authentication configuration").into(),
        )
    }

    fn get_name(&self) -> String {
        "sumup".to_string()
    }
}

static CONNECTOR: SumupTest = SumupTest {};

// Provides default payment information for tests
fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        amount: Some(1000), // e.g., 10.00 EUR in minor units
        currency: Some(enums::Currency::EUR),
        connector_auth_type: None, // Will be fetched by SumupTest::get_auth_token
        country: Some(api_models::enums::CountryAlpha2::DE), // Example country
        metadata: None,
        payment_method_type: Some(enums::PaymentMethodType::Card),
        payment_id: Some(types::PaymentIdType::PaymentIntentId("test_payment_id".to_string())),
        shipping_address: None,
        billing_address: None,
        email: Some(Secret::new("test@example.com".to_string())),
        return_url: Some("https://hyperswitch.io/test_return_url".to_string()),
        setup_future_usage: None,
        ..Default::default()
    })
}

// Provides card details for tests (though not directly used in the first SumUp auth call body)
fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(types::api::Card {
            card_number: cards::CardNumber::try_from("4000000000000001".to_string()).unwrap(), // Example card
            card_exp_month: Secret::new("12".to_string()),
            card_exp_year: Secret::new("2030".to_string()),
            card_cvc: Secret::new("123".to_string()),
            card_holder_name: Some(Secret::new("Test User".to_string())),
            ..Default::default()
        }),
        amount: 1000, // Minor units
        currency: enums::Currency::EUR,
        capture_method: Some(enums::CaptureMethod::Automatic), // SumUp typically auto-captures
        confirm: true,
        statement_descriptor_suffix: None,
        statement_descriptor: None,
        setup_future_usage: None,
        mandate_id: None,
        off_session: None,
        browser_info: None,
        order_details: None,
        customer_id: Some("test_customer_sumup".to_string()),
        router_return_url: Some("https://hyperswitch.io/test_return_url".to_string()),
        webhook_url: Some("https://hyperswitch.io/test_webhook_url".to_string()),
        email: Some(Secret::new("test@example.com".to_string())),
        metadata: None, // checkout_id will be populated here by the connector
        connector_meta: None,
        payment_experience: None,
        payment_method_type: Some(enums::PaymentMethodType::Card),
        session_token: None,
        client_secret: None,
        customer_acceptance: None,
        setup_mandate_details: None,
        authentication_type: None,
        merchant_connector_details: None,
        order_category: None,
        customer_initiated_authentication: None,
        surcharge_details: None,
        frm_metadata: None,
        customer_details: None,
        shipping_address: None,
        billing_address: None,
        card_cvc: None,
        mandate_data: None,
        payment_method_data_billing_address: None,
        request_incremental_authorization: false,
        merchant_initiated_card_on_file_payment: None,
        customer_id_from_input: None,
        idempotency_key: None,
    })
}

// Cards Positive Tests
// Test for the first step of SumUp authorization (POST /v0.1/checkouts)
#[actix_web::test]
async fn should_initiate_authorize_payment() {
    let authorize_data = payment_method_details();
    let payment_info = get_default_payment_info();

    // The CONNECTOR.authorize_payment will internally call the refactored Authorize flow
    let response = CONNECTOR
        .authorize_payment(authorize_data.as_ref(), payment_info.as_ref())
        .await
        .expect("Authorize payment (checkout creation) response");

    // Assert status indicates further action (e.g. payment details submission)
    assert_eq!(response.status, enums::AttemptStatus::RequiresCustomerAction);
    
    // Assert that the response contains the checkout_id (as resource_id and in connector_metadata)
    match response.response {
        Ok(types::PaymentsResponseData::TransactionResponse { 
            resource_id, 
            connector_metadata, 
            .. 
        }) => {
            let checkout_id_from_meta = connector_metadata
                .as_ref()
                .and_then(|meta| meta.get("checkout_id"))
                .and_then(|val| val.as_str());
            
            assert!(checkout_id_from_meta.is_some(), "checkout_id not found in connector_metadata");
            if let types::ResponseId::ConnectorTransactionId(id) = resource_id {
                 assert_eq!(id, checkout_id_from_meta.unwrap(), "resource_id does not match checkout_id from metadata");
            } else {
                panic!("resource_id is not ConnectorTransactionId");
            }
        }
        _ => panic!("Response was not a successful TransactionResponse"),
    }
}

// Test for checking status of an auto-captured payment via Capture flow
#[actix_web::test]
async fn should_get_auto_captured_payment_status() {
    // First, simulate a full payment that should be auto-captured
    let initial_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Initial make_payment call failed");

    // Ensure initial payment is charged (or whatever status make_payment returns for success)
    // This depends on how make_payment is implemented for SumUp's two-step flow.
    // For now, we assume it would return Charged if successful.
    // If it returns RequiresCustomerAction, this test needs a different setup.
    assert_eq!(initial_response.status, enums::AttemptStatus::Charged, "Initial payment was not charged by make_payment");

    let connector_txn_id = utils::get_connector_transaction_id(initial_response.response)
        .expect("Failed to get connector_transaction_id from make_payment response");

    // Now, call the Capture flow, which for SumUp is a GET to check status
    let capture_response = CONNECTOR
        .capture_payment(
            connector_txn_id,
            None, // No specific capture amount for a status check
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment (status check) response");
    
    // Assert that the status is Charged
    assert_eq!(capture_response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
// This test might not be directly applicable if SumUp always auto-captures 
// or doesn't support partial capture for the initial authorization.
// Keeping it as a placeholder, might need adjustment based on SumUp's specific behavior.
#[actix_web::test]
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

// Synchronizes a payment status.
#[actix_web::test]
async fn should_sync_payment_status() {
    // First, simulate a full payment that should be auto-captured
    let initial_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Initial make_payment call failed");

    // Assuming make_payment results in a Charged status for SumUp if successful
    assert_eq!(initial_response.status, enums::AttemptStatus::Charged, "Initial payment was not charged by make_payment");
    
    let connector_txn_id = utils::get_connector_transaction_id(initial_response.response)
        .expect("Failed to get connector_transaction_id from make_payment response");

    // Now, call PSync
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged, // Expecting Charged status
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(connector_txn_id),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Voids a checkout before it's processed with payment details.
#[actix_web::test]
async fn should_void_checkout() {
    // Step 1: Initiate authorization (creates a checkout)
    let authorize_response_result = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await;
    
    assert!(authorize_response_result.is_ok(), "Checkout creation failed");
    let authorize_response = authorize_response_result.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::RequiresCustomerAction, "Checkout creation did not result in RequiresCustomerAction");

    let checkout_id = match authorize_response.response {
        Ok(types::PaymentsResponseData::TransactionResponse { resource_id, .. }) => {
            match resource_id {
                types::ResponseId::ConnectorTransactionId(id) => id,
                _ => panic!("Resource ID is not ConnectorTransactionId"),
            }
        },
        _ => panic!("Authorize response was not a successful TransactionResponse with resource_id"),
    };

    // Step 2: Void the checkout
    let void_response_result = CONNECTOR
        .void_payment(
            checkout_id.clone(), // Pass the checkout_id as the ID to void
            Some(types::PaymentsCancelData {
                connector_transaction_id: checkout_id, // SumUp's DEL /v0.1/checkouts/{id} uses checkout_id
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await;

    assert!(void_response_result.is_ok(), "Void payment call failed");
    let void_response = void_response_result.unwrap();
    
    // Assert that the status is Voided
    // This depends on how the void_payment helper and the connector's handle_response for Void are implemented.
    // If SumUp returns 204 for successful void, the status should be Voided.
    assert_eq!(void_response.status, enums::AttemptStatus::Voided, "Payment status was not Voided after void call");
}

// Creates a refund for a successfully captured payment.
#[actix_web::test]
async fn should_create_refund() {
    // This helper simulates a payment and then a full refund.
    // It should internally handle the two-step auth for SumUp if `make_payment` is correctly set up.
    let response = CONNECTOR
        .make_payment_and_refund(payment_method_details(), None, get_default_payment_info())
        .await
        .expect("Refund creation failed");

    // SumUp's refund create (POST /v0.1/me/refund/{txn_id}) returns 204 No Content.
    // The test helper `make_payment_and_refund` should reflect this.
    // The `RefundsResponseData` will be constructed with status Success if 204 is received.
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
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

// Synchronizes a refund status.
#[actix_web::test]
async fn should_sync_refund_status() {
    // Create a payment and a full refund
    let refund_create_response = CONNECTOR
        .make_payment_and_refund(payment_method_details(), None, get_default_payment_info())
        .await
        .expect("make_payment_and_refund failed");

    let initial_refund_data = refund_create_response.response.expect("Refund data missing from refund create response");
    assert_eq!(initial_refund_data.refund_status, enums::RefundStatus::Success, "Initial refund status was not Success");

    // Sync the refund
    let sync_response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success, // Expecting Success status
            initial_refund_data.connector_refund_id,
            None, // No specific refund data for sync
            get_default_payment_info(),
        )
        .await
        .expect("Refund Sync response");
    
    assert_eq!(
        sync_response.response.unwrap().refund_status,
        enums::RefundStatus::Success
    );
}

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_make_payment() {
    let authorize_response = CONNECTOR.make_payment(payment_method_details(), get_default_payment_info()).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR.make_payment(payment_method_details(), get_default_payment_info()).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(
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
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
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
async fn should_refund_succeeded_payment_multiple_times() {
    CONNECTOR
        .make_payment_and_multiple_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await;
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
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
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Cards Negative scenarios
// Creates a payment with incorrect CVC.
#[actix_web::test]
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
        "Your card's security code is invalid.".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: api::PaymentMethodData::Card(api::Card {
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
        "Your card's expiration month is invalid.".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: api::PaymentMethodData::Card(api::Card {
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
        response.response.unwrap_err().message,
        "Your card's expiration year is invalid.".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR.make_payment(payment_method_details(), get_default_payment_info()).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "You cannot cancel this PaymentIntent because it has a status of succeeded."
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("No such payment_intent: '123456789'")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
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
