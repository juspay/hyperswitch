use api_models::{webhook_events, webhooks};
use common_utils::{crypto::SignMessage, ext_traits::Encode};
use error_stack::ResultExt;
use masking::Secret;
use serde::Serialize;

use crate::{
    core::errors,
    headers, logger,
    services::request::Maskable,
    types::storage::{self, enums},
};

#[derive(Debug)]
pub enum ScheduleWebhookRetry {
    WithProcessTracker(Box<storage::ProcessTracker>),
    NoSchedule,
}

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
    pub(crate) merchant_id: common_utils::id_type::MerchantId,
    pub(crate) business_profile_id: common_utils::id_type::ProfileId,
    pub(crate) event_type: enums::EventType,
    pub(crate) event_class: enums::EventClass,
    pub(crate) primary_object_id: String,
    pub(crate) primary_object_type: enums::EventObjectType,
    pub(crate) initial_attempt_id: Option<String>,
}

pub struct WebhookResponse {
    pub response: reqwest::Response,
}

impl WebhookResponse {
    pub async fn get_outgoing_webhook_response_content(
        self,
    ) -> webhook_events::OutgoingWebhookResponseContent {
        let status_code = self.response.status();
        let response_headers = self
            .response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    value
                        .to_str()
                        .map(|s| Secret::from(String::from(s)))
                        .unwrap_or_else(|error| {
                            logger::warn!(
                                "Response header {} contains non-UTF-8 characters: {error:?}",
                                name.as_str()
                            );
                            Secret::from(String::from("Non-UTF-8 header value"))
                        }),
                )
            })
            .collect::<Vec<_>>();
        let response_body = self
            .response
            .text()
            .await
            .map(Secret::from)
            .unwrap_or_else(|error| {
                logger::warn!("Response contains non-UTF-8 characters: {error:?}");
                Secret::from(String::from("Non-UTF-8 response body"))
            });
        webhook_events::OutgoingWebhookResponseContent {
            body: Some(response_body),
            headers: Some(response_headers),
            status_code: Some(status_code.as_u16()),
            error_message: None,
        }
    }
}
