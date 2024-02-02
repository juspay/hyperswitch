use api_models::webhooks;
use common_utils::{crypto::SignMessage, ext_traits};
use error_stack::ResultExt;
use serde::Serialize;

use crate::{core::errors, headers, services::request::Maskable};

pub trait OutgoingWebhookType:
    Serialize + From<webhooks::OutgoingWebhook> + Sync + Send + std::fmt::Debug + 'static
{
    fn get_outgoing_webhooks_signature(
        &self,
        payment_response_hash_key: Option<String>,
    ) -> errors::CustomResult<Option<String>, errors::WebhooksFlowError>;

    fn add_webhook_header(header: &mut Vec<(String, Maskable<String>)>, signature: String);
}

impl OutgoingWebhookType for webhooks::OutgoingWebhook {
        /// This method generates the signature for outgoing webhooks using the provided payment response hash key.
    /// It encodes the self object to a JSON string, then uses the provided hash key to sign the encoded payload
    /// using HMAC-SHA512 algorithm. The resulting signature is then optionally encoded to a hex string and
    /// returned as an Option. If the encoding or signing process fails, it returns a WebhooksFlowError.
    fn get_outgoing_webhooks_signature(
        &self,
        payment_response_hash_key: Option<String>,
    ) -> errors::CustomResult<Option<String>, errors::WebhooksFlowError> {
        let webhook_signature_payload =
            ext_traits::Encode::<serde_json::Value>::encode_to_string_of_json(self)
                .change_context(errors::WebhooksFlowError::OutgoingWebhookEncodingFailed)
                .attach_printable("failed encoding outgoing webhook payload")?;

        Ok(payment_response_hash_key
            .map(|key| {
                common_utils::crypto::HmacSha512::sign_message(
                    &common_utils::crypto::HmacSha512,
                    key.as_bytes(),
                    webhook_signature_payload.as_bytes(),
                )
            })
            .transpose()
            .change_context(errors::WebhooksFlowError::OutgoingWebhookSigningFailed)
            .attach_printable("Failed to sign the message")?
            .map(hex::encode))
    }
        /// Adds a webhook signature header to the provided Vec of headers.
    ///
    /// # Arguments
    ///
    /// * `header` - A mutable reference to a Vec of tuples containing strings and Maskable<String> types
    /// * `signature` - The signature to be added to the header
    ///
    fn add_webhook_header(header: &mut Vec<(String, Maskable<String>)>, signature: String) {
        header.push((headers::X_WEBHOOK_SIGNATURE.to_string(), signature.into()))
    }
}
