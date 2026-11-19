use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
use common_utils::{crypto::Encryptable, id_type::MerchantId, pii::SecretSerdeValue};
use hyperswitch_connectors::connectors::{Braintree, Checkout};
use hyperswitch_interfaces::{
    errors::ConnectorError,
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use hyperswitch_masking::Secret;
use ring::hmac;
use sha1::{Digest, Sha1};

fn webhook_details(secret: &str) -> Option<SecretSerdeValue> {
    Some(Secret::new(serde_json::json!({
        "merchant_secret": secret,
        "additional_secret": "public_key"
    })))
}

async fn verify(
    connector: &dyn IncomingWebhook,
    body: &[u8],
    headers: &HeaderMap,
    details: Option<SecretSerdeValue>,
) -> common_utils::errors::CustomResult<bool, ConnectorError> {
    connector
        .verify_webhook_source(
            &IncomingWebhookRequestDetails {
                method: http::Method::POST,
                uri: http::Uri::from_static("/webhooks/test"),
                headers,
                body,
                query_params: String::new(),
            },
            &MerchantId::get_irrelevant_merchant_id(),
            details,
            Encryptable::new(Secret::new(serde_json::json!({})), Secret::new(Vec::new())),
            connector.id(),
        )
        .await
}

#[actix_web::test]
async fn checkout_webhook_secret_rejects_forgery_and_preserves_valid_signatures() {
    let body = br#"{"type":"payment_approved","data":{"id":"pay_existing"}}"#;
    for secret in ["", " \t\n", "\u{2003}", "valid_secret", " valid_secret\n"] {
        let signature = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes()), body);
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("cko-signature"),
            HeaderValue::from_str(&hex::encode(signature.as_ref())).expect("hex signature"),
        );
        let result = verify(Checkout::new(), body, &headers, webhook_details(secret)).await;
        if secret.trim().is_empty() {
            // This exact error context is converted to source_verified=false by
            // the webhook gateway, allowing the payment flow to sync instead.
            assert!(matches!(
                result
                    .expect_err("blank key signature must not verify")
                    .current_context(),
                ConnectorError::WebhookSourceVerificationFailed
            ));
        } else {
            assert!(result.expect("valid signature verifies"));
            for additional_secret in ["", " \t\n"] {
                let result = verify(
                    Checkout::new(),
                    body,
                    &headers,
                    Some(Secret::new(serde_json::json!({
                        "merchant_secret": secret,
                        "additional_secret": additional_secret
                    }))),
                )
                .await;
                assert!(matches!(
                    result
                        .expect_err("blank additional secret must not verify")
                        .current_context(),
                    ConnectorError::WebhookSourceVerificationFailed
                ));
            }
            assert!(!verify(
                Checkout::new(),
                b"tampered",
                &headers,
                webhook_details(secret)
            )
            .await
            .expect("signature mismatch"));
        }
    }
}

#[actix_web::test]
async fn braintree_webhook_secret_rejects_forgery_and_preserves_valid_signatures() {
    let payload = "dGVzdA==";
    for secret in ["", " \t\n", "\u{2003}", "valid_secret", " valid_secret\n"] {
        let key = Sha1::digest(secret.as_bytes());
        let signature = hmac::sign(
            &hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, key.as_slice()),
            payload.as_bytes(),
        );
        let body = serde_urlencoded::to_string([
            (
                "bt_signature",
                format!("public_key|{}", hex::encode(signature.as_ref())),
            ),
            ("bt_payload", payload.to_string()),
        ])
        .expect("webhook form body");
        let result = verify(
            Braintree::new(),
            body.as_bytes(),
            &HeaderMap::new(),
            webhook_details(secret),
        )
        .await;
        if secret.trim().is_empty() {
            assert!(matches!(
                result
                    .expect_err("blank key signature must not verify")
                    .current_context(),
                ConnectorError::WebhookSourceVerificationFailed
            ));
        } else {
            assert!(result.expect("valid signature verifies"));
            assert!(!verify(
                Braintree::new(),
                body.as_bytes(),
                &HeaderMap::new(),
                webhook_details("wrong_secret")
            )
            .await
            .expect("signature mismatch"));
        }
    }
}

#[actix_web::test]
async fn missing_webhook_secret_fails_source_verification() {
    for connector in [Checkout::new() as &dyn IncomingWebhook, Braintree::new()] {
        for details in [
            None,
            Some(Secret::new(
                serde_json::json!({"additional_secret": "public_key"}),
            )),
        ] {
            let result = verify(connector, b"{}", &HeaderMap::new(), details).await;
            assert!(matches!(
                result
                    .expect_err("missing secret must not verify")
                    .current_context(),
                ConnectorError::WebhookSourceVerificationFailed
            ));
        }
    }
}
