use std::str::FromStr;

use api_models::{webhook_events, webhooks};
use common_utils::{crypto::SignMessage, ext_traits::Encode};
use error_stack::ResultExt;
use hyperswitch_domain_models::router_response_types::NotifyConnectorResponseData;
use hyperswitch_masking::Secret;
use serde::Serialize;

use crate::{
    core::errors,
    headers, logger,
    services::request::Maskable,
    types::{
        api::OutgoingWebhookContent,
        domain::MerchantConnectorAccount,
        storage::{self, enums},
        MinorUnit,
    },
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

/// Tracking data serialized into the process tracker for outgoing webhook retries.
///
/// This data is persisted so that the retry workflow can reconstruct the correct
/// context (merchant account, keystore, business profile) for webhook redelivery.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct OutgoingWebhookTrackingData {
    /// The provider (business owner) merchant id. Always populated.
    pub(crate) merchant_id: common_utils::id_type::MerchantId,
    /// The business profile of the webhook recipient (initiator's profile).
    pub(crate) business_profile_id: common_utils::id_type::ProfileId,
    /// The merchant_id of the merchant whose connector credentials were used for payment processing.
    /// In standard setups this equals `merchant_id`.
    pub(crate) processor_merchant_id: Option<common_utils::id_type::MerchantId>,
    /// The merchant_id of the webhook recipient (the merchant that initiated the
    /// operation). Used during retries to look up the correct keystore directly.
    /// `None` for tracking data created by older deployments (falls back to `merchant_id`).
    #[serde(default)]
    pub(crate) initiator_merchant_id: Option<common_utils::id_type::MerchantId>,
    pub(crate) event_type: enums::EventType,
    pub(crate) event_class: enums::EventClass,
    pub(crate) primary_object_id: String,
    pub(crate) primary_object_type: enums::EventObjectType,
    pub(crate) initial_attempt_id: Option<String>,
    pub(crate) recipient_data: Option<WebhookRecipientData>,
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

/// Unified interface for webhook delivery responses from different sources.
///
/// Implemented for [`reqwest::Response`] (merchant webhook path) and
/// [`NotifyConnectorResponseData`] (UCS connector notification path).
#[async_trait::async_trait]
pub(crate) trait WebhookDeliveryResponse: Send {
    fn status(&self) -> u16;
    fn is_success(&self) -> bool;
    fn get_response_headers(&self) -> Vec<(String, Secret<String>)>;
    fn get_error_message(&self) -> Option<String>;
    async fn get_response_body(self) -> Secret<String>;
}

#[async_trait::async_trait]
impl WebhookDeliveryResponse for reqwest::Response {
    fn status(&self) -> u16 {
        self.status().as_u16()
    }

    fn is_success(&self) -> bool {
        self.status().is_success()
    }

    fn get_response_headers(&self) -> Vec<(String, Secret<String>)> {
        self.headers()
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
            .collect::<Vec<_>>()
    }

    async fn get_response_body(self) -> Secret<String> {
        self.text().await.map(Secret::from).unwrap_or_else(|error| {
            logger::warn!("Response contains non-UTF-8 characters: {error:?}");
            Secret::from(String::from("Non-UTF-8 response body"))
        })
    }

    fn get_error_message(&self) -> Option<String> {
        None
    }
}

#[async_trait::async_trait]
impl WebhookDeliveryResponse for NotifyConnectorResponseData {
    fn status(&self) -> u16 {
        self.status_code
    }

    fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }

    fn get_response_headers(&self) -> Vec<(String, Secret<String>)> {
        vec![]
    }

    fn get_error_message(&self) -> Option<String> {
        self.error_message.clone()
    }

    async fn get_response_body(self) -> Secret<String> {
        Secret::from(serde_json::to_string(&self).unwrap_or_else(|error| {
            logger::warn!("Failed to serialize response: {error:?}");
            String::from("Failed to serialize response")
        }))
    }
}

pub(crate) struct MerchantWebhook;
pub(crate) struct ConnectorWebhook;
pub(crate) struct InitialAttempt;
pub(crate) struct AutomaticRetry;
pub(crate) struct ManualRetry;

pub(crate) struct WebhookPayload {
    pub event_type: enums::EventType,
    pub event_content: Option<OutgoingWebhookContent>,
    pub recipient_data: WebhookRecipientData,
}

impl WebhookPayload {
    /// Builds a surcharge webhook payload if the primary event supports surcharge notification
    /// and the payment attempt has external surcharge details.
    #[cfg(feature = "v1")]
    pub fn build_surcharge_payload(
        surcharge_event: enums::EventType,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
        merchant_surcharge_connector: &MerchantConnectorAccount,
    ) -> errors::CustomResult<Option<Self>, errors::ApiErrorResponse> {
        let connector = common_enums::connector_enums::Connector::from_str(
            &merchant_surcharge_connector.connector_name,
        )
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector",
        })?;

        Ok(payment_attempt
            .external_surcharge_details
            .as_ref()
            .map(|external_surcharge_details| {
                let feature_specific_data = SurchargeDetails {
                    surcharge_amount: external_surcharge_details.external_surcharge_amount,
                    external_surcharge_id: external_surcharge_details.external_surcharge_id.clone(),
                    payment_id: payment_attempt.payment_id.clone(),
                    attempt_id: payment_attempt.attempt_id.clone(),
                };
                Self {
                    event_type: surcharge_event,
                    event_content: None,
                    recipient_data: WebhookRecipientData::Connector {
                        connector,
                        merchant_connector_id: merchant_surcharge_connector
                            .merchant_connector_id
                            .clone(),
                        feature_data: FeatureTrackingData::SurchargeDetails(Box::new(
                            feature_specific_data,
                        )),
                    },
                }
            }))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum WebhookRecipientData {
    Merchant {
        merchant_id: common_utils::id_type::MerchantId,
    },
    Connector {
        connector: common_enums::connector_enums::Connector,
        merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
        feature_data: FeatureTrackingData,
    },
}

impl WebhookRecipientData {
    pub fn get_event_recipient(&self) -> common_enums::EventRecipient {
        match self {
            Self::Merchant { .. } => common_enums::EventRecipient::Merchant,
            Self::Connector { .. } => common_enums::EventRecipient::Connector,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureTrackingData {
    SurchargeDetails(Box<SurchargeDetails>),
}

/// Details of surcharge applied on this payment, if applicable
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SurchargeDetails {
    pub surcharge_amount: MinorUnit,
    pub external_surcharge_id: String,
    pub payment_id: common_utils::id_type::PaymentId,
    pub attempt_id: String,
}
