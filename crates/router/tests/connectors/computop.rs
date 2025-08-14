use std::str::FromStr;

use api_models::{enums as api_enums, payments as api_payments};
use cards::CardNumber;
use common_enums::enums;
use common_utils::{pii::SecretSerdeValue, types::MinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::{Card, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_request_types::PaymentsAuthorizeData,
    router_response_types::PaymentsResponseData,
    types,
};
use hyperswitch_interfaces::api::{self, ConnectorIntegration, ConnectorValidation};
use masking::Secret;
use router::connector::Computop;
use serde_json::Value;
use time::OffsetDateTime;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentInfo},
};

fn get_default_payment_info() -> Option<PaymentInfo> {
    Some(PaymentInfo {
        address: None,
        auth_type: None,
        connector_meta_data: None,
        return_url: Some("https://hyperswitch.io".to_string()),
    })
}

fn payment_method_details() -> Option<PaymentMethodData> {
    Some(PaymentMethodData::Card(Card {
        card_number: CardNumber::from_str("4111111111111111").ok()?,
        card_exp_month: Secret::new("12".to_string()),
        card_exp_year: Secret::new("2025".to_string()),
        card_holder_name: Some(Secret::new("John Doe".to_string())),
        card_cvc: Some(Secret::new("123".to_string())),
        card_issuer: None,
        card_network: None,
        card_type: None,
        card_issuing_country: None,
        bank_code: None,
        nick_name: Some(Secret::new("nick_name".to_string())),
    }))
}

fn get_connector_transaction_id(response: PaymentsResponseData) -> Option<String> {
    match response {
        PaymentsResponseData::TransactionResponse { resource_id, .. } => {
            resource_id.get_connector_transaction_id().ok()
        }
        _ => None,
    }
}

static CONNECTOR: Computop = Computop {};

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the capture_payment method.
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let txn_id = get_connector_transaction_id(response.response);
    assert!(txn_id.is_some());
    let response = CONNECTOR
        .capture_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the capture_payment method.
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let txn_id = get_connector_transaction_id(response.response);
    assert!(txn_id.is_some());
    let response = CONNECTOR
        .capture_payment(txn_id.unwrap(), Some(MinorUnit::new(50)), get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the sync_payment method.
#[actix_web::test]
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = get_connector_transaction_id(authorize_response.response);
    let response = CONNECTOR
        .psync_payment(None, txn_id, get_default_payment_info())
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Voids a payment using the void_payment method.
#[actix_web::test]
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let txn_id = get_connector_transaction_id(response.response);
    let response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the refund_payment method.
#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let txn_id = get_connector_transaction_id(response.response);
    let response = CONNECTOR
        .capture_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
    let txn_id = get_connector_transaction_id(response.response);
    let response = CONNECTOR
        .refund_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .expect("Refund payment response");
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the refund_payment method.
#[actix_web::test]
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let txn_id = get_connector_transaction_id(response.response);
    let response = CONNECTOR
        .capture_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
    let txn_id = get_connector_transaction_id(response.response);
    let response = CONNECTOR
        .refund_payment(txn_id.unwrap(), Some(MinorUnit::new(50)), get_default_payment_info())
        .await
        .expect("Refund payment response");
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Synchronizes a refund using the sync_refund method.
#[actix_web::test]
async fn should_sync_refund() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    let txn_id = get_connector_transaction_id(response.response);
    let response = CONNECTOR
        .capture_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
    let txn_id = get_connector_transaction_id(response.response);
    let response = CONNECTOR
        .refund_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .expect("Refund payment response");
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
    let response = CONNECTOR
        .rsync_refund(response.response.unwrap().connector_refund_id, get_default_payment_info())
        .await
        .expect("Refund sync response");
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Make payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the sync_payment method.
#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Make payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = get_connector_transaction_id(authorize_response.response);
    assert!(txn_id.is_some());
    let response = CONNECTOR
        .psync_payment(None, txn_id, get_default_payment_info())
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Refunds a payment using the refund_payment method.
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Make payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
    let txn_id = get_connector_transaction_id(response.response);
    let response = CONNECTOR
        .refund_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .expect("Refund payment response");
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Cards Negative scenarios
// Creates a payment with incorrect CVC.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let mut payment_method_details = payment_method_details();
    if let Some(PaymentMethodData::Card(ref mut card)) = payment_method_details {
        card.card_cvc = Some(Secret::new("12".to_string()));
    }
    let response = CONNECTOR
        .make_payment(payment_method_details, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Failure,);
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_month() {
    let mut payment_method_details = payment_method_details();
    if let Some(PaymentMethodData::Card(ref mut card)) = payment_method_details {
        card.card_exp_month = Secret::new("20".to_string());
    }
    let response = CONNECTOR
        .make_payment(payment_method_details, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Failure,);
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let mut payment_method_details = payment_method_details();
    if let Some(PaymentMethodData::Card(ref mut card)) = payment_method_details {
        card.card_exp_year = Secret::new("2000".to_string());
    }
    let response = CONNECTOR
        .make_payment(payment_method_details, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Failure,);
}

// Voids a payment using an incorrect connector transaction id.
#[actix_web::test]
async fn should_fail_void_payment_for_incorrect_connector_transaction_id() {
    let response = CONNECTOR
        .void_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Failure,);
}

// Captures a payment using an incorrect connector transaction id.
#[actix_web::test]
async fn should_fail_capture_payment_for_incorrect_connector_transaction_id() {
    let response = CONNECTOR
        .capture_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Failure,);
}

// Refunds a payment using an incorrect connector transaction id.
#[actix_web::test]
async fn should_fail_refund_payment_for_incorrect_connector_transaction_id() {
    let response = CONNECTOR
        .refund_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Invalid transaction id".to_string(),
    );
}

// Synchronizes a payment using an incorrect connector transaction id.
#[actix_web::test]
async fn should_fail_sync_payment_for_incorrect_connector_transaction_id() {
    let response = CONNECTOR
        .psync_payment(None, Some("123456789".to_string()), get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Failure,);
}