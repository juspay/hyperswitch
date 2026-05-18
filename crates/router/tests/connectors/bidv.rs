use hyperswitch_domain_models::payment_method_data::{BankTransferData, PaymentMethodData};
use router::types::{self, api, storage::enums};
use test_utils::connector_auth;

use crate::utils::{self, ConnectorActions};

#[derive(Clone, Copy)]
struct BidvTest;
impl ConnectorActions for BidvTest {}
impl utils::Connector for BidvTest {
    fn get_data(&self) -> api::ConnectorData {
        use router::connector::Bidv;
        utils::construct_connector_data_old(
            Box::new(Bidv::new()),
            types::Connector::Bidv,
            api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .bidv
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "bidv".to_string()
    }
}

static CONNECTOR: BidvTest = BidvTest {};

/// Base metadata required by all BIDV payment requests.
fn base_metadata() -> serde_json::Value {
    serde_json::json!({
        "merchant_id": "TEST_MERCHANT",
        "merchant_name": "Test Merchant",
        "channel_id": "WEB",
        "account_number": "1234567890",
    })
}

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    None
}

/// Authorize data for the corporate / business bank-transfer flow.
fn business_payment_details() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: PaymentMethodData::BankTransfer(Box::new(
            BankTransferData::LocalBankTransfer { bank_code: None },
        )),
        router_return_url: Some("https://hyperswitch.io/return".to_string()),
        webhook_url: Some("https://hyperswitch.io/webhook".to_string()),
        currency: enums::Currency::VND,
        metadata: Some({
            let mut m = base_metadata();
            m["account_type"] = serde_json::Value::String("business".to_string());
            m
        }),
        ..utils::PaymentAuthorizeType::default().0
    })
}

/// Authorize data for the personal / e-wallet bank-transfer flow.
fn personal_payment_details() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: PaymentMethodData::BankTransfer(Box::new(
            BankTransferData::LocalBankTransfer { bank_code: None },
        )),
        router_return_url: Some("https://hyperswitch.io/return".to_string()),
        webhook_url: Some("https://hyperswitch.io/webhook".to_string()),
        currency: enums::Currency::VND,
        customer_name: Some(hyperswitch_masking::Secret::new("Nguyen Van A".to_string())),
        metadata: Some({
            let mut m = base_metadata();
            m["account_type"] = serde_json::Value::String("personal".to_string());
            m["payer_id"] = serde_json::Value::String("PAYER001".to_string());
            m
        }),
        ..utils::PaymentAuthorizeType::default().0
    })
}

// ---- Business (corporate paygate) flow ----

/// Initiates a corporate bank-transfer; BIDV redirects the customer to its payment portal.
#[actix_web::test]
async fn should_initiate_business_bank_transfer() {
    let response = CONNECTOR
        .authorize_payment(business_payment_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
}

/// Syncs a corporate bank-transfer by the connector transaction id returned at initiation.
#[actix_web::test]
async fn should_sync_business_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(business_payment_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(
        authorize_response.status,
        enums::AttemptStatus::AuthenticationPending
    );
    let txn_id = utils::get_connector_transaction_id(authorize_response.response)
        .expect("Missing connector transaction id");
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::AuthenticationPending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(txn_id),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
}

// ---- Personal (e-wallet) flow ----

/// Initiates a personal e-wallet bank-transfer; BIDV redirects the customer.
#[actix_web::test]
async fn should_initiate_personal_bank_transfer() {
    let response = CONNECTOR
        .authorize_payment(personal_payment_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
}

/// Syncs a personal e-wallet payment by its connector transaction id.
#[actix_web::test]
async fn should_sync_personal_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(personal_payment_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(
        authorize_response.status,
        enums::AttemptStatus::AuthenticationPending
    );
    let txn_id = utils::get_connector_transaction_id(authorize_response.response)
        .expect("Missing connector transaction id");
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::AuthenticationPending,
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(txn_id),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::AuthenticationPending);
}

// ---- Error / negative scenarios ----

/// A payment with no metadata should fail because merchant_id is required.
#[actix_web::test]
async fn should_fail_payment_without_metadata() {
    let response = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: PaymentMethodData::BankTransfer(Box::new(
                    BankTransferData::LocalBankTransfer { bank_code: None },
                )),
                currency: enums::Currency::VND,
                metadata: None,
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Authorize payment response");
    assert!(
        response.response.is_err(),
        "Expected error when metadata is missing"
    );
}

/// A personal-flow payment without payer_id should fail.
#[actix_web::test]
async fn should_fail_personal_payment_without_payer_id() {
    let response = CONNECTOR
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: PaymentMethodData::BankTransfer(Box::new(
                    BankTransferData::LocalBankTransfer { bank_code: None },
                )),
                currency: enums::Currency::VND,
                customer_name: Some(hyperswitch_masking::Secret::new(
                    "Nguyen Van A".to_string(),
                )),
                metadata: Some({
                    let mut m = base_metadata();
                    m["account_type"] = serde_json::Value::String("personal".to_string());
                    // payer_id intentionally omitted
                    m
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Authorize payment response");
    assert!(
        response.response.is_err(),
        "Expected error when payer_id is missing for personal flow"
    );
}
