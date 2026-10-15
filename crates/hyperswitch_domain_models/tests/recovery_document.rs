#![cfg(all(feature = "v2", feature = "revenue_recovery"))]
// Test fixtures and assertions intentionally fail fast on invalid data.
#![allow(clippy::unwrap_used)]

use std::borrow::Cow;

use api_models::payments::{PaymentsCreateIntentRequest, RecoveryPaymentsCreate};
use common_utils::{
    encryption::Encryption,
    types::keymanager::{Identifier, KeyManagerState, ToEncryptable},
};
use hyperswitch_domain_models::{
    payments::{
        payment_intent::CustomerData, EncryptedPaymentIntent, FromRequestEncryptablePaymentIntent,
    },
    revenue_recovery::RevenueRecoveryInvoiceData,
    type_encryption::{crypto_operation, CryptoOperation},
};
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};

fn request(document: Option<serde_json::Value>) -> RecoveryPaymentsCreate {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/recovery_payment.json")).unwrap();
    if let Some(document) = document {
        value.as_object_mut().unwrap().insert(
            "customer_details".to_string(),
            serde_json::json!({"email": "customer@example.com", "document_details": document}),
        );
    }
    serde_json::from_value(value).unwrap()
}

fn invoice(request: RecoveryPaymentsCreate) -> RevenueRecoveryInvoiceData {
    RevenueRecoveryInvoiceData {
        amount: request.amount_details.order_amount().into(),
        currency: request.amount_details.currency(),
        merchant_reference_id: request.merchant_reference_id,
        billing_address: request.billing,
        customer_details: request.customer_details,
        retry_count: None,
        next_billing_at: None,
        billing_started_at: Some(request.billing_started_at),
        metadata: request.metadata,
        enable_partial_authorization: request.enable_partial_authorization,
    }
}

fn document() -> serde_json::Value {
    serde_json::json!({"document_type": "cpf", "document_number": "12345678909"})
}

#[test]
fn document_and_billing_survive_recovery_intent_conversion_without_customer() {
    let invoice = invoice(request(Some(document())));
    let intent = PaymentsCreateIntentRequest::from(&invoice);
    assert!(intent.customer_id.is_none());
    let customer = intent.customer_details.unwrap();
    assert_eq!(customer.email.unwrap().peek(), "customer@example.com");
    let doc = customer.document_details.unwrap();
    doc.validate().unwrap();
    assert_eq!(doc.document_number.peek(), "12345678909");
    let billing = intent.billing.unwrap();
    assert_eq!(billing.email.unwrap().peek(), "payer@example.com");
    assert_eq!(
        billing.address.unwrap().first_name.unwrap().peek(),
        "Diagnostic"
    );
}

#[test]
fn document_is_optional_for_existing_recovery_requests() {
    let intent = PaymentsCreateIntentRequest::from(&invoice(request(None)));
    assert!(intent.customer_details.is_none());
    assert!(intent.customer_id.is_none());
}

#[test]
fn combined_v2_payment_request_preserves_document_on_intent_creation() {
    let intent = PaymentsCreateIntentRequest::from(&invoice(request(Some(document()))));
    let combined: api_models::payments::PaymentsRequest =
        serde_json::from_value(serde_json::json!({
            "amount_details": {"order_amount": 1000, "currency": "BRL"},
            "customer_details": {"email": "customer@example.com", "document_details": document()},
            "payment_method_type": "card",
            "payment_method_data": {"card": {"card_number": "4242424242424242", "card_exp_month": "12", "card_exp_year": "2030", "card_cvc": "123"}}
        })).unwrap();
    let recreated = PaymentsCreateIntentRequest::from(&combined);
    assert_eq!(recreated.customer_details, intent.customer_details);
}

#[test]
fn document_requires_both_type_and_number_and_uses_existing_cpf_validation() {
    for invalid in [
        serde_json::json!({"document_type": "cpf"}),
        serde_json::json!({"document_number": "12345678909"}),
    ] {
        assert!(
            serde_json::from_value::<api_models::customers::CustomerDocumentDetails>(invalid)
                .is_err()
        );
    }
    for number in ["", "11111111111"] {
        let doc: api_models::customers::CustomerDocumentDetails = serde_json::from_value(
            serde_json::json!({"document_type": "cpf", "document_number": number}),
        )
        .unwrap();
        assert!(doc.validate().is_err());
    }
}

#[test]
fn document_snapshot_round_trips_through_intent_batch_encryption() {
    futures::executor::block_on(async {
        let request = request(Some(document()));
        let customer = CustomerData::from(request.customer_details.unwrap());
        let customer_json = serde_json::to_value(&customer).unwrap();
        let state = KeyManagerState::mock();
        let key = [42u8; 32];
        let identifier = Identifier::Merchant(
            common_utils::id_type::MerchantId::try_from(Cow::Borrowed("recovery_document_test"))
                .unwrap(),
        );
        let encrypted = crypto_operation(
            &state,
            "PaymentIntent",
            CryptoOperation::BatchEncrypt(FromRequestEncryptablePaymentIntent::to_encryptable(
                FromRequestEncryptablePaymentIntent {
                    customer_details: Some(Secret::new(customer_json.clone())),
                    billing_address: request
                        .billing
                        .map(|billing| Secret::new(serde_json::to_value(billing).unwrap())),
                    shipping_address: None,
                    recipient_details: None,
                },
            )),
            identifier.clone(),
            &key,
        )
        .await
        .unwrap()
        .try_into_batchoperation()
        .unwrap();
        let encrypted = FromRequestEncryptablePaymentIntent::from_encryptable(encrypted).unwrap();
        let stored = Encryption::from(encrypted.customer_details.unwrap());
        assert!(!stored
            .clone()
            .into_inner()
            .expose()
            .windows(11)
            .any(|bytes| bytes == b"12345678909"));
        assert!(!stored
            .clone()
            .into_inner()
            .expose()
            .windows(b"customer@example.com".len())
            .any(|bytes| bytes == b"customer@example.com"));
        let decrypted = crypto_operation(
            &state,
            "PaymentIntent",
            CryptoOperation::<serde_json::Value, _>::BatchDecrypt(
                EncryptedPaymentIntent::to_encryptable(EncryptedPaymentIntent {
                    customer_details: Some(stored),
                    billing_address: encrypted.billing_address.map(Encryption::from),
                    shipping_address: None,
                    recipient_details: None,
                }),
            ),
            identifier,
            &key,
        )
        .await
        .unwrap()
        .try_into_batchoperation()
        .unwrap();
        let decrypted = EncryptedPaymentIntent::from_encryptable(decrypted).unwrap();
        let restored = decrypted.customer_details.unwrap().into_inner().expose();
        assert_eq!(restored, customer_json);
        let restored: CustomerData = serde_json::from_value(restored).unwrap();
        assert_eq!(restored.email.unwrap().peek(), "customer@example.com");
        assert_eq!(
            restored
                .customer_document_details
                .unwrap()
                .document_number
                .peek(),
            "12345678909"
        );
        assert_eq!(
            decrypted
                .billing_address
                .unwrap()
                .into_inner()
                .expose()
                .get("email")
                .unwrap(),
            "payer@example.com"
        );
    });
}

#[test]
fn absent_customer_email_stays_absent_even_with_billing_email() {
    let request = request(None);
    assert!(request.billing.unwrap().email.is_some());
    let customer = CustomerData::from(api_models::payments::CustomerDetails::default());
    assert!(customer.email.is_none());
    assert!(customer.customer_document_details.is_none());
}
