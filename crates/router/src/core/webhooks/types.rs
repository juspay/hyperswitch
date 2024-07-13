use api_models::webhooks;
use common_utils::{crypto::SignMessage, ext_traits::Encode};
use error_stack::ResultExt;
use masking::Secret;
use serde::Serialize;

use crate::{core::errors, headers, services::request::Maskable, types::storage::enums};

pub struct OutgoingWebhookPayloadWithSignature {
    pub payload: Secret<String>,
    pub signature: Option<String>,
}

pub trait OutgoingWebhookType:
    Serialize + From<webhooks::OutgoingWebhook> + Sync + Send + std::fmt::Debug + 'static
{
    fn get_outgoing_webhooks_signature(
        &self,
        payment_response_hash_key: Option<impl AsRef<[u8]>>,
    ) -> errors::CustomResult<OutgoingWebhookPayloadWithSignature, errors::WebhooksFlowError>;

    fn add_webhook_header(header: &mut Vec<(String, Maskable<String>)>, signature: String);
}

impl OutgoingWebhookType for webhooks::OutgoingWebhook {
    fn get_outgoing_webhooks_signature(
        &self,
        payment_response_hash_key: Option<impl AsRef<[u8]>>,
    ) -> errors::CustomResult<OutgoingWebhookPayloadWithSignature, errors::WebhooksFlowError> {
        let webhook_signature_payload = self
            .encode_to_string_of_json()
            .change_context(errors::WebhooksFlowError::OutgoingWebhookEncodingFailed)
            .attach_printable("failed encoding outgoing webhook payload")?;

        let signature = payment_response_hash_key
            .map(|key| {
                common_utils::crypto::HmacSha512::sign_message(
                    &common_utils::crypto::HmacSha512,
                    key.as_ref(),
                    webhook_signature_payload.as_bytes(),
                )
            })
            .transpose()
            .change_context(errors::WebhooksFlowError::OutgoingWebhookSigningFailed)
            .attach_printable("Failed to sign the message")?
            .map(hex::encode);

        Ok(OutgoingWebhookPayloadWithSignature {
            payload: webhook_signature_payload.into(),
            signature,
        })
    }

    fn add_webhook_header(header: &mut Vec<(String, Maskable<String>)>, signature: String) {
        header.push((headers::X_WEBHOOK_SIGNATURE.to_string(), signature.into()))
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct OutgoingWebhookTrackingData {
    pub(crate) merchant_id: String,
    pub(crate) business_profile_id: String,
    pub(crate) event_type: enums::EventType,
    pub(crate) event_class: enums::EventClass,
    pub(crate) primary_object_id: String,
    pub(crate) primary_object_type: enums::EventObjectType,
    pub(crate) initial_attempt_id: Option<String>,
}
