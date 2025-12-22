use hyperswitch_domain_models::router_flow_types::payments;
use hyperswitch_domain_models::router_flow_types::refunds;
use hyperswitch_domain_models::router_request_types;
use hyperswitch_domain_models::router_response_types;
use hyperswitch_domain_models::types;
use hyperswitch_interfaces::api;
use hyperswitch_interfaces::errors;
use hyperswitch_interfaces::webhooks;
use hyperswitch_router::connector;
use hyperswitch_router::core::errors;
use hyperswitch_router::core::payments;
use hyperswitch_router::core::refunds;
use hyperswitch_router::types::api;
use hyperswitch_router::types::domain;
use hyperswitch_router::types::storage;
use hyperswitch_router::types:: transformers::ForeignFrom;
use hyperswitch_router::types:: transformers::ForeignInto;
use hyperswitch_router::types::{self, Response as RouterResponse};
use hyperswitch_router::utils;
use masking::Secret;
use serde_json::Value;
use url::Url;

use crate::test_utils::get_connector;

#[tokio::test]
async fn getnetglobal_payment_authorize() {
    let connector = get_connector("getnetglobal");

    let payment_authorize_data = payments::AuthorizeData {
        amount: 100,
        currency: "USD",
        payment_method: "card",
        payment_method_data: router_request_types::PaymentMethodData::Card(
            router_request_types::Card {
                card_number: Secret::new("4111111111111111".to_string()),
                card_exp_month: Secret::new("12".to_string()),
                card_exp_year: Secret::new("25".to_string()),
                card_cvc: Secret::new("123".to_string()),
                card_holder_name: Some(Secret::new("John Doe".to_string())),
            },
        ),
        customer_id: Some("customer_123".to_string()),
        email: Some("user@example.com".to_string()),
        name: Some("John Doe".to_string()),
        ..Default::default()
    };

    let router_data = types::RouterData::foreign_from((
        payment_authorize_data,
        types::PaymentsAuthorizeRequestData,
    ));

    let response = connector
        .authorize(&router_data)
        .await
        .expect("Authorization failed");

    assert!(response.response.is_ok());
    assert_eq!(response.response.unwrap().status, "AUTHORIZED");
}

#[tokio::test]
async fn getnetglobal_payment_capture() {
    let connector = get_connector("getnetglobal");

    let payment_capture_data = payments::CaptureData {
        amount: 100,
        currency: "USD",
        connector_transaction_id: "txn_123".to_string(),
        ..Default::default()
    };

    let router_data = types::RouterData::foreign_from((
        payment_capture_data,
        types::PaymentsCaptureRequestData,
    ));

    let response = connector
        .capture(&router_data)
        .await
        .expect("Capture failed");

    assert!(response.response.is_ok());
    assert_eq!(response.response.unwrap().status, "CAPTURED");
}

#[tokio::test]
async fn getnetglobal_payment_void() {
    let connector = get_connector("getnetglobal");

    let payment_void_data = payments::VoidData {
        amount: 100,
        currency: "USD",
        connector_transaction_id: "txn_123".to_string(),
        ..Default::default()
    };

    let router_data = types::RouterData::foreign_from((
        payment_void_data,
        types::PaymentsCancelRequestData,
    ));

    let response = connector
        .void(&router_data)
        .await
        .expect("Void failed");

    assert!(response.response.is_ok());
    assert_eq!(response.response.unwrap().status, "VOIDED");
}

#[tokio::test]
async fn getnetglobal_payment_sync() {
    let connector = get_connector("getnetglobal");

    let payment_sync_data = payments::SyncData {
        connector_transaction_id: "txn_123".to_string(),
        ..Default::default()
    };

    let router_data = types::RouterData::foreign_from((
        payment_sync_data,
        types::PaymentsSyncRequestData,
    ));

    let response = connector
        .psync(&router_data)
        .await
        .expect("Sync failed");

    assert!(response.response.is_ok());
    assert_eq!(response.response.unwrap().status, "AUTHORIZED");
}

#[tokio::test]
async fn getnetglobal_refund() {
    let connector = get_connector("getnetglobal");

    let refund_data = refunds::RefundData {
        amount: 100,
        currency: "USD",
        connector_transaction_id: "txn_123".to_string(),
        refund_id: "refund_123".to_string(),
        ..Default::default()
    };

    let router_data = types::RouterData::foreign_from((
        refund_data,
        types::RefundsRequestData,
    ));

    let response = connector
        .refund(&router_data)
        .await
        .expect("Refund failed");

    assert!(response.response.is_ok());
    assert_eq!(response.response.unwrap().status, "REFUNDED");
}

#[tokio::test]
async fn getnetglobal_refund_sync() {
    let connector = get_connector("getnetglobal");

    let refund_sync_data = refunds::RefundSyncData {
        connector_refund_id: "refund_123".to_string(),
        connector_transaction_id: "txn_123".to_string(),
        ..Default::default()
    };

    let router_data = types::RouterData::foreign_from((
        refund_sync_data,
        types::RefundsSyncRequestData,
    ));

    let response = connector
        .rsync(&router_data)
        .await
        .expect("Refund sync failed");

    assert!(response.response.is_ok());
    assert_eq!(response.response.unwrap().status, "REFUNDED");
}

#[tokio::test]
async fn getnetglobal_webhook_verification() {
    let connector = get_connector("getnetglobal");

    // Test webhook payload and signature
    let webhook_payload = r#"{
        "response_base64": "eyJ0cmFuc2FjdGlvbl9pZCI6ICJ0eG5fMTIzIiwgInRyYW5zYWN0aW9uX3R5cGUiOiAiUEFZRk1FUiIsICJ0cmFuc2FjdGlvbl9zdGF0ZSI6ICJBVVRIQVJJWkVEIn0=",
        "response_signature_base64": "aW52YWxpZCBzaWduYXR1cmU="
    }"#;

    let result = connector
        .verify_webhook_source(
            &webhooks::IncomingWebhookRequestDetails {
                body: webhook_payload.as_bytes(),
                ..Default::default()
            },
            &"merchant_id".into(),
            None,
            crypto::Encryptable::new(Secret::new(serde_json::Value::Null)),
            "getnetglobal",
        )
        .await;

    assert!(result.is_err());
    if let Err(err) = result {
        assert_eq!(err.current_context(), &errors::ConnectorError::WebhookSourceVerificationFailed);
    }
}

#[tokio::test]
async fn getnetglobal_webhook_payment_event() {
    let connector = get_connector("getnetglobal");

    let webhook_payload = r#"{
        "response_base64": "eyJ0cmFuc2FjdGlvbl9pZCI6ICJ0eG5fMTIzIiwgInRyYW5zYWN0aW9uX3R5cGUiOiAiUEFZRk1FUiIsICJ0cmFuc2FjdGlvbl9zdGF0ZSI6ICJBVVRIQVJJWkVEIn0=",
        "response_signature_base64": "c3VjY2Vzc2Z1bCBzaWduYXR1cmU="
    }"#;

    let event = connector
        .get_webhook_event_type(&webhooks::IncomingWebhookRequestDetails {
            body: webhook_payload.as_bytes(),
            ..Default::default()
        })
        .expect("Failed to get webhook event");

    assert_eq!(event, webhooks::IncomingWebhookEvent::PaymentSucceeded);
}

#[tokio::test]
async fn getnetglobal_webhook_refund_event() {
    let connector = get_connector("getnetglobal");

    let webhook_payload = r#"{
        "response_base64": "eyJ0cmFuc2FjdGlvbl9pZCI6ICJyZm5fMTIzIiwgInRyYW5zYWN0aW9uX3R5cGUiOiAiUkVGVU5EIiwgInRyYW5zYWN0aW9uX3N0YXRlIjogIkNPTVBMRVRFRCJ9",
        "response_signature_base64": "c3VjY2Vzc2Z1bCBzaWduYXR1cmU="
    }"#;

    let event = connector
        .get_webhook_event_type(&webhooks::IncomingWebhookRequestDetails {
            body: webhook_payload.as_bytes(),
            ..Default::default()
        })
        .expect("Failed to get webhook event");

    assert_eq!(event, webhooks::IncomingWebhookEvent::RefundSucceeded);
}