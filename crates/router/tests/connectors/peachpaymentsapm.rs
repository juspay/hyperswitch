use hyperswitch_domain_models::payment_method_data::{BankTransferData, PaymentMethodData};
use router::types::{self, api, storage::enums};
use test_utils::connector_auth;

use crate::utils::{self, ConnectorActions};

#[derive(Clone, Copy)]
struct PeachpaymentsapmTest;
impl ConnectorActions for PeachpaymentsapmTest {}
impl utils::Connector for PeachpaymentsapmTest {
    fn get_data(&self) -> api::ConnectorData {
        use router::connector::Peachpaymentsapm;
        utils::construct_connector_data_old(
            Box::new(Peachpaymentsapm::new()),
            types::Connector::Peachpaymentsapm,
            api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .peachpaymentsapm
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "peachpaymentsapm".to_string()
    }
}

static CONNECTOR: PeachpaymentsapmTest = PeachpaymentsapmTest {};

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    None
}

// Helper function to create PayShap payment data
fn get_payshap_payment_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        amount: 10000, // 100.00 ZAR in minor units
        currency: enums::Currency::ZAR,
        payment_method_data: PaymentMethodData::BankTransfer(Box::new(
            BankTransferData::LocalBankTransfer {
                bank_code: Some("PAYSHAP".to_string()),
            },
        )),
        confirm: true,
        capture_method: Some(enums::CaptureMethod::Automatic),
        router_return_url: Some("https://example.com/return".to_string()),
        webhook_url: Some("https://example.com/webhook".to_string()),
        ..utils::PaymentAuthorizeType::default().0
    })
}

// Helper function to create Capitec Pay payment data
fn get_capitec_pay_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        amount: 10000, // 100.00 ZAR in minor units
        currency: enums::Currency::ZAR,
        payment_method_data: PaymentMethodData::BankTransfer(Box::new(
            BankTransferData::LocalBankTransfer {
                bank_code: Some("CAPITECPAY".to_string()),
            },
        )),
        confirm: true,
        capture_method: Some(enums::CaptureMethod::Automatic),
        router_return_url: Some("https://example.com/return".to_string()),
        webhook_url: Some("https://example.com/webhook".to_string()),
        ..utils::PaymentAuthorizeType::default().0
    })
}

// Helper function to create Peach EFT payment data
fn get_peach_eft_data() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        amount: 10000, // 100.00 ZAR in minor units
        currency: enums::Currency::ZAR,
        payment_method_data: PaymentMethodData::BankTransfer(Box::new(
            BankTransferData::LocalBankTransfer {
                bank_code: Some("PEACH_EFT".to_string()),
            },
        )),
        confirm: true,
        capture_method: Some(enums::CaptureMethod::Automatic),
        router_return_url: Some("https://example.com/return".to_string()),
        webhook_url: Some("https://example.com/webhook".to_string()),
        ..utils::PaymentAuthorizeType::default().0
    })
}

// ============================================================================
// Bank Transfer Positive Tests
// ============================================================================
// These tests verify the connector integration with PeachPayments APM API.
// They are designed to run against a real Hyperswitch server connected to
// PeachPayments sandbox environment.
//
// To run these tests:
// 1. Start the Hyperswitch server: cargo run
// 2. Configure connector credentials in connector_auth.toml
// 3. Run: cargo test --package router --test connectors -- peachpaymentsapm --include-ignored
// ============================================================================

// PayShap: Initiates a PayShap payment (async with redirect flow)
#[ignore = "Tested via Cypress E2E tests"]
#[actix_web::test]
async fn should_initiate_payshap_payment() {
    let response = CONNECTOR
        .make_payment(get_payshap_payment_data(), get_default_payment_info())
        .await
        .unwrap();
    // PayShap is an async payment method that requires redirect
    // Initial status should be AuthenticationPending or Pending
    assert!(
        response.status == enums::AttemptStatus::AuthenticationPending
            || response.status == enums::AttemptStatus::Pending
            || response.status == enums::AttemptStatus::Charged,
        "Unexpected status: {:?}",
        response.status
    );
}

// Capitec Pay: Initiates a Capitec Pay payment (async with redirect flow)
#[ignore = "Tested via Cypress E2E tests"]
#[actix_web::test]
async fn should_initiate_capitec_pay_payment() {
    let response = CONNECTOR
        .make_payment(get_capitec_pay_data(), get_default_payment_info())
        .await
        .unwrap();
    // Capitec Pay is an async payment method that requires redirect
    assert!(
        response.status == enums::AttemptStatus::AuthenticationPending
            || response.status == enums::AttemptStatus::Pending
            || response.status == enums::AttemptStatus::Charged,
        "Unexpected status: {:?}",
        response.status
    );
}

// Peach EFT: Initiates a Peach EFT payment (async with redirect flow)
#[ignore = "Tested via Cypress E2E tests"]
#[actix_web::test]
async fn should_initiate_peach_eft_payment() {
    let response = CONNECTOR
        .make_payment(get_peach_eft_data(), get_default_payment_info())
        .await
        .unwrap();
    // Peach EFT is an async payment method that requires redirect
    assert!(
        response.status == enums::AttemptStatus::AuthenticationPending
            || response.status == enums::AttemptStatus::Pending
            || response.status == enums::AttemptStatus::Charged,
        "Unexpected status: {:?}",
        response.status
    );
}

// Synchronizes a pending payment
#[ignore = "Tested via Cypress E2E tests"]
#[actix_web::test]
async fn should_sync_pending_payment() {
    let authorize_response = CONNECTOR
        .make_payment(get_payshap_payment_data(), get_default_payment_info())
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone());

    if let Some(txn_id) = txn_id {
        let response = CONNECTOR
            .sync_payment(
                Some(types::PaymentsSyncData {
                    connector_transaction_id: types::ResponseId::ConnectorTransactionId(txn_id),
                    ..Default::default()
                }),
                get_default_payment_info(),
            )
            .await
            .unwrap();
        // Payment sync should return a valid status
        assert!(
            response.status == enums::AttemptStatus::AuthenticationPending
                || response.status == enums::AttemptStatus::Pending
                || response.status == enums::AttemptStatus::Charged
                || response.status == enums::AttemptStatus::Failure,
            "Unexpected sync status: {:?}",
            response.status
        );
    }
}

// ============================================================================
// Negative Test Cases
// ============================================================================

// Test with invalid bank code
#[ignore = "Tested via Cypress E2E tests"]
#[actix_web::test]
async fn should_fail_with_invalid_bank_code() {
    let payment_data = Some(types::PaymentsAuthorizeData {
        amount: 10000,
        currency: enums::Currency::ZAR,
        payment_method_data: PaymentMethodData::BankTransfer(Box::new(
            BankTransferData::LocalBankTransfer {
                bank_code: Some("INVALID_BANK".to_string()),
            },
        )),
        confirm: true,
        capture_method: Some(enums::CaptureMethod::Automatic),
        router_return_url: Some("https://example.com/return".to_string()),
        ..utils::PaymentAuthorizeType::default().0
    });

    let response = CONNECTOR
        .make_payment(payment_data, get_default_payment_info())
        .await
        .unwrap();
    // Should fail with invalid bank code
    assert!(
        response.status == enums::AttemptStatus::Failure || response.response.is_err(),
        "Expected failure for invalid bank code"
    );
}

// Test with invalid currency (non-ZAR for South African EFT)
#[ignore = "Tested via Cypress E2E tests"]
#[actix_web::test]
async fn should_fail_with_invalid_currency() {
    let payment_data = Some(types::PaymentsAuthorizeData {
        amount: 10000,
        currency: enums::Currency::USD, // South African EFT only supports ZAR
        payment_method_data: PaymentMethodData::BankTransfer(Box::new(
            BankTransferData::LocalBankTransfer {
                bank_code: Some("PAYSHAP".to_string()),
            },
        )),
        confirm: true,
        capture_method: Some(enums::CaptureMethod::Automatic),
        router_return_url: Some("https://example.com/return".to_string()),
        ..utils::PaymentAuthorizeType::default().0
    });

    let response = CONNECTOR
        .make_payment(payment_data, get_default_payment_info())
        .await
        .unwrap();
    // Should fail with invalid currency
    assert!(
        response.status == enums::AttemptStatus::Failure || response.response.is_err(),
        "Expected failure for invalid currency"
    );
}

// Test sync with invalid transaction ID
#[ignore = "Tested via Cypress E2E tests"]
#[actix_web::test]
async fn should_fail_sync_with_invalid_transaction_id() {
    let response = CONNECTOR
        .sync_payment(
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                    "invalid_transaction_id_12345".to_string(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    // Should return failure or error for invalid transaction ID
    assert!(
        response.status == enums::AttemptStatus::Failure || response.response.is_err(),
        "Expected failure for invalid transaction ID"
    );
}
