//! ACI webhook notifications: decrypting them, and what they report.
//!
//! ACI encrypts every notification, its "Click to Test" activation ping included, with
//! AES-256-GCM under the webhook's 64-hex-character secret: the body is the ciphertext in hex
//! (bare, or wrapped as `{"encryptedBody": "<hex>"}`), and the IV and authentication tag travel
//! hex-encoded in the `X-Initialization-Vector` and `X-Authentication-Tag` headers.

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId, RefundIdType};
use common_utils::{
    errors::{CryptoError, CustomResult},
    pii::Email,
    types::StringMajorUnit,
};
use error_stack::{report, ResultExt};
use hyperswitch_interfaces::errors::ConnectorError;
use hyperswitch_masking::Secret;
use ring::aead::{self, UnboundKey};
use serde::{Deserialize, Serialize};

use super::{
    response::AciResponseResultDetails,
    result::{ResultClass, ResultCode},
    three_ds::AciThreeDSecureResponse,
};

/// ACI sends webhook event types in UPPERCASE (e.g. "PAYMENT", "REGISTRATION").
///
/// The `Test` variant covers ACI's "webhook activation" ping ("Click to Test"), which has a
/// lowercase `type` of `"test"`. It must decrypt and be acknowledged with a 2xx for ACI to
/// activate the webhook.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AciWebhookEventType {
    Payment,
    Registration,
    Schedule,
    Risk,
    #[serde(rename = "test", alias = "TEST")]
    Test,
}

/// ACI sends action values in UPPERCASE (e.g. "CREATED", "UPDATED", "DELETED").
///
/// `Other` is a catch-all so that unrecognised actions (e.g. the test webhook's
/// `"webhook activation"`) do not fail deserialization of the whole notification.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AciWebhookAction {
    Created,
    Updated,
    Deleted,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciWebhookCardDetails {
    pub bin: Option<String>,
    #[serde(rename = "last4Digits")]
    pub last4_digits: Option<String>,
    pub holder: Option<String>,
    pub expiry_month: Option<Secret<String>>,
    pub expiry_year: Option<Secret<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciWebhookCustomerDetails {
    #[serde(rename = "givenName")]
    pub given_name: Option<Secret<String>>,
    pub surname: Option<Secret<String>>,
    #[serde(rename = "merchantCustomerId")]
    pub merchant_customer_id: Option<Secret<String>>,
    pub sex: Option<Secret<String>>,
    pub email: Option<Email>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciWebhookAuthenticationDetails {
    #[serde(rename = "entityId")]
    pub entity_id: Secret<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciWebhookRiskDetails {
    pub score: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciPaymentWebhookPayload {
    pub id: String,
    pub payment_type: String,
    pub payment_brand: String,
    pub amount: StringMajorUnit,
    pub currency: String,
    pub presentation_amount: Option<StringMajorUnit>,
    pub presentation_currency: Option<String>,
    pub descriptor: Option<String>,
    pub result: ResultCode,
    pub authentication: Option<AciWebhookAuthenticationDetails>,
    pub card: Option<AciWebhookCardDetails>,
    pub customer: Option<AciWebhookCustomerDetails>,
    #[serde(rename = "customParameters")]
    pub custom_parameters: Option<serde_json::Value>,
    pub risk: Option<AciWebhookRiskDetails>,
    pub build_number: Option<String>,
    pub timestamp: String,
    pub ndc: String,
    #[serde(rename = "channelName")]
    pub channel_name: Option<String>,
    pub source: Option<String>,
    pub payment_method: Option<String>,
    #[serde(rename = "shortId")]
    pub short_id: Option<String>,
    /// Registration token ID returned when `createRegistration=true` was sent.
    pub registration_id: Option<Secret<String>>,
    /// Acquirer / connector-level result details.
    pub result_details: Option<AciResponseResultDetails>,
    /// 3DS authentication data from ACI
    #[serde(rename = "threeDSecure")]
    pub three_d_secure: Option<AciThreeDSecureResponse>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciWebhookNotification {
    #[serde(rename = "type")]
    pub event_type: AciWebhookEventType,
    pub action: Option<AciWebhookAction>,
    pub payload: serde_json::Value,
}

impl AciWebhookNotification {
    /// The object the notification is about: a refund for `paymentType=RF`, a payment otherwise.
    /// `None` without a payload `id`.
    pub fn object_reference_id(&self) -> Option<ObjectReferenceId> {
        let id = self.payload.get("id")?.as_str()?.to_string();
        let is_refund = self
            .payload
            .get("paymentType")
            .and_then(serde_json::Value::as_str)
            .is_some_and(is_refund_payment_type);
        Some(if is_refund {
            ObjectReferenceId::RefundId(RefundIdType::ConnectorRefundId(id))
        } else {
            ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(id),
            )
        })
    }

    /// The payload of a `PAYMENT` notification. `None` for any other type.
    pub fn into_payment_payload(
        self,
    ) -> Option<Result<AciPaymentWebhookPayload, serde_json::Error>> {
        match self.event_type {
            AciWebhookEventType::Payment => Some(serde_json::from_value(self.payload)),
            // Registration webhooks are for mandate/token lifecycle events; Schedule and Risk
            // webhooks are not currently actionable; Test is ACI's "webhook activation"
            // connectivity ping. All are acknowledged without further processing.
            AciWebhookEventType::Registration
            | AciWebhookEventType::Schedule
            | AciWebhookEventType::Risk
            | AciWebhookEventType::Test => None,
        }
    }
}

impl AciPaymentWebhookPayload {
    /// The event a payment notification reports: its result class, for a payment or a refund.
    pub fn event(&self) -> IncomingWebhookEvent {
        let is_refund = is_refund_payment_type(&self.payment_type);
        match (ResultClass::of(&self.result.code), is_refund) {
            (Some(ResultClass::Success), false) => IncomingWebhookEvent::PaymentIntentSuccess,
            (Some(ResultClass::Success), true) => IncomingWebhookEvent::RefundSuccess,
            (Some(ResultClass::Pending), false) => IncomingWebhookEvent::PaymentIntentProcessing,
            (Some(ResultClass::Failure), false) => IncomingWebhookEvent::PaymentIntentFailure,
            (Some(ResultClass::Failure), true) => IncomingWebhookEvent::RefundFailure,
            (Some(ResultClass::Pending), true) | (None, _) => {
                IncomingWebhookEvent::EventNotSupported
            }
        }
    }
}

fn is_refund_payment_type(payment_type: &str) -> bool {
    payment_type.eq_ignore_ascii_case("RF")
}

/// A webhook body as it arrives, told apart by the encryption headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AciWebhookBody<'a> {
    /// No encryption headers. Passed through as it came, for a plaintext notification to be
    /// acknowledged, but never trusted: nothing proves who sent it.
    Plaintext(&'a [u8]),
    /// Encrypted under the webhook secret.
    Encrypted {
        iv: &'a str,
        auth_tag: &'a str,
        body: &'a [u8],
    },
}

impl<'a> AciWebhookBody<'a> {
    /// The body, given the `X-Initialization-Vector` and `X-Authentication-Tag` header values.
    /// One without the other is neither a plaintext nor a decryptable notification.
    pub fn of(
        iv: Option<&'a str>,
        auth_tag: Option<&'a str>,
        body: &'a [u8],
    ) -> CustomResult<Self, ConnectorError> {
        match (iv, auth_tag) {
            (Some(iv), Some(auth_tag)) => Ok(Self::Encrypted { iv, auth_tag, body }),
            (None, None) => Ok(Self::Plaintext(body)),
            (Some(_), None) | (None, Some(_)) => Err(report!(
                ConnectorError::WebhookBodyDecodingFailed
            ))
            .attach_printable(
                "ACI webhook carries only one of X-Initialization-Vector and X-Authentication-Tag",
            ),
        }
    }

    /// Whether the body proves its source. An encrypted body does once [`Self::decode`] has
    /// succeeded: the GCM tag only verifies under the webhook secret, so a body forged or altered
    /// without it fails to decode and never gets this far.
    pub fn is_authenticated(&self) -> bool {
        matches!(self, Self::Encrypted { .. })
    }

    /// The notification JSON, decrypted with `secret` (the webhook secret, as hex).
    pub fn decode(self, secret: &[u8]) -> CustomResult<Vec<u8>, ConnectorError> {
        let (iv, auth_tag, body) = match self {
            Self::Plaintext(body) => return Ok(body.to_vec()),
            Self::Encrypted { iv, auth_tag, body } => (iv, auth_tag, body),
        };
        let key = std::str::from_utf8(secret)
            .map_err(|_| report!(ConnectorError::WebhookVerificationSecretInvalid))
            .attach_printable("ACI webhook secret is not a valid UTF-8 string")?;
        let body = std::str::from_utf8(body)
            .map_err(|_| report!(ConnectorError::WebhookBodyDecodingFailed))
            .attach_printable("Encrypted ACI webhook body is not UTF-8")?;
        let ciphertext = serde_json::from_str::<AciEncryptedWebhookBody>(body)
            .map(|wrapped| wrapped.encrypted_body)
            .unwrap_or_else(|_| body.trim().to_string());
        decrypt_payload(key.trim(), iv.trim(), auth_tag.trim(), &ciphertext)
            .change_context(ConnectorError::WebhookBodyDecodingFailed)
            .attach_printable("Failed to decrypt ACI webhook payload")
    }
}

/// The JSON wrapper a webhook can be configured to deliver its ciphertext in.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AciEncryptedWebhookBody {
    encrypted_body: String,
}

/// Decrypts an AES-256-GCM encrypted payload where the IV, auth tag, and ciphertext are provided
/// separately as hex strings, as ACI sends its webhooks.
///
/// # Arguments
/// * `hex_key`: The encryption key as a hex string (must decode to 32 bytes).
/// * `hex_iv`: The initialization vector (nonce) as a hex string (must decode to 12 bytes).
/// * `hex_auth_tag`: The authentication tag as a hex string (must decode to 16 bytes).
/// * `hex_encrypted_body`: The encrypted payload as a hex string.
fn decrypt_payload(
    hex_key: &str,
    hex_iv: &str,
    hex_auth_tag: &str,
    hex_encrypted_body: &str,
) -> CustomResult<Vec<u8>, CryptoError> {
    let key_bytes = hex::decode(hex_key)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decode hex key")?;
    let iv_bytes = hex::decode(hex_iv)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decode hex IV")?;
    let auth_tag_bytes = hex::decode(hex_auth_tag)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decode hex auth tag")?;
    let encrypted_body_bytes = hex::decode(hex_encrypted_body)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decode hex encrypted body")?;
    if key_bytes.len() != 32 {
        return Err(CryptoError::InvalidKeyLength)
            .attach_printable("Key must be 32 bytes for AES-256-GCM");
    }
    if iv_bytes.len() != aead::NONCE_LEN {
        return Err(CryptoError::InvalidIvLength)
            .attach_printable(format!("IV must be {} bytes for AES-GCM", aead::NONCE_LEN));
    }
    if auth_tag_bytes.len() != 16 {
        return Err(CryptoError::InvalidTagLength)
            .attach_printable("Auth tag must be 16 bytes for AES-256-GCM");
    }

    let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to create unbound key")?;

    let less_safe_key = aead::LessSafeKey::new(unbound_key);

    let nonce_arr: [u8; aead::NONCE_LEN] = iv_bytes
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::InvalidIvLength)
        .attach_printable_lazy(|| {
            format!(
                "IV length is {} but expected {}",
                iv_bytes.len(),
                aead::NONCE_LEN
            )
        })?;
    let nonce = aead::Nonce::assume_unique_for_key(nonce_arr);

    let mut ciphertext_and_tag = encrypted_body_bytes;
    ciphertext_and_tag.extend_from_slice(&auth_tag_bytes);

    less_safe_key
        .open_in_place(nonce, aead::Aad::empty(), &mut ciphertext_and_tag)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decrypt payload using LessSafeKey")?;

    let original_ciphertext_len = ciphertext_and_tag.len() - auth_tag_bytes.len();
    ciphertext_and_tag.truncate(original_ciphertext_len);

    Ok(ciphertext_and_tag)
}

#[cfg(test)]
mod tests {

    use super::*;

    /// ACI's "webhook activation" ping has a lowercase `type` of `"test"` and an
    /// `action` of `"webhook activation"`.
    /// It must deserialize into an [`AciWebhookNotification`] (rather than
    /// failing on the unknown `type`/`action` values) so the connector can
    /// acknowledge it. See the `Test` and `Other` catch-all variants above.
    #[test]
    fn test_webhook_activation_notification_deserializes() {
        let body = r#"{
            "type": "test",
            "action": "webhook activation",
            "payload": {
                "result": {
                    "code": "000.100.110",
                    "description": "Request successfully processed in 'Merchant in Integrator Test Mode'"
                }
            }
        }"#;

        let notification: AciWebhookNotification =
            serde_json::from_str(body).expect("test webhook should deserialize");

        assert_eq!(notification.event_type, AciWebhookEventType::Test);
        assert_eq!(notification.action, Some(AciWebhookAction::Other));
    }

    /// An uppercase `"TEST"` type is also tolerated via the serde alias.
    #[test]
    fn test_webhook_uppercase_type_alias() {
        let body = r#"{"type": "TEST", "payload": {}}"#;
        let notification: AciWebhookNotification =
            serde_json::from_str(body).expect("uppercase TEST type should deserialize");
        assert_eq!(notification.event_type, AciWebhookEventType::Test);
    }

    fn payment(payment_type: &str, code: &str) -> AciPaymentWebhookPayload {
        serde_json::from_value(serde_json::json!({
            "id": "8ac7a4a1",
            "paymentType": payment_type,
            "paymentBrand": "VISA",
            "amount": "10.00",
            "currency": "ZAR",
            "result": {"code": code, "description": "described"},
            "timestamp": "t",
            "ndc": "n",
        }))
        .expect("payment payload deserializes")
    }

    #[test]
    fn payment_events_follow_the_result_class_for_payments_and_refunds() {
        let cases = [
            (
                "DB",
                "000.100.110",
                IncomingWebhookEvent::PaymentIntentSuccess,
            ),
            (
                "PA",
                "000.200.000",
                IncomingWebhookEvent::PaymentIntentProcessing,
            ),
            (
                "DB",
                "800.100.151",
                IncomingWebhookEvent::PaymentIntentFailure,
            ),
            ("rf", "000.100.110", IncomingWebhookEvent::RefundSuccess),
            ("RF", "000.200.000", IncomingWebhookEvent::EventNotSupported),
            ("RF", "800.100.151", IncomingWebhookEvent::RefundFailure),
            ("DB", "123.456.789", IncomingWebhookEvent::EventNotSupported),
        ];
        for (payment_type, code, expected) in cases {
            assert_eq!(
                payment(payment_type, code).event(),
                expected,
                "{payment_type} {code}"
            );
        }
    }

    #[test]
    fn a_refund_notification_references_the_refund() {
        let notification = |payment_type: &str| AciWebhookNotification {
            event_type: AciWebhookEventType::Payment,
            action: None,
            payload: serde_json::json!({"id": "8ac7a4a1", "paymentType": payment_type}),
        };
        assert!(matches!(
            notification("RF").object_reference_id(),
            Some(ObjectReferenceId::RefundId(_))
        ));
        assert!(matches!(
            notification("DB").object_reference_id(),
            Some(ObjectReferenceId::PaymentId(_))
        ));
        let anonymous = AciWebhookNotification {
            payload: serde_json::json!({}),
            ..notification("DB")
        };
        assert!(anonymous.object_reference_id().is_none());
    }

    #[test]
    fn only_payment_notifications_carry_a_payment_payload() {
        let ping: AciWebhookNotification =
            serde_json::from_str(r#"{"type": "test", "payload": {}}"#).expect("ping deserializes");
        assert!(ping.into_payment_payload().is_none());
    }

    #[test]
    fn decryption_rejects_malformed_inputs() {
        let key = hex::encode([7u8; 32]);
        let iv = hex::encode([9u8; aead::NONCE_LEN]);
        let tag = hex::encode([0u8; 16]);
        assert!(decrypt_payload("zz", &iv, &tag, "00").is_err());
        assert!(decrypt_payload(&hex::encode([7u8; 16]), &iv, &tag, "00").is_err());
        assert!(decrypt_payload(&key, &hex::encode([9u8; 8]), &tag, "00").is_err());
        assert!(decrypt_payload(&key, &iv, &hex::encode([0u8; 8]), "00").is_err());
        assert!(
            decrypt_payload(&key, &iv, &tag, "00").is_err(),
            "a wrong tag fails to open"
        );
    }
}
