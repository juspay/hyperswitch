use api_models::{
    enums::{DisputeStatus, MandateStatus},
    webhooks::{self as api},
};
use common_utils::{crypto::SignMessage, date_time, ext_traits};
use error_stack::{IntoReport, ResultExt};
use router_env::logger;
use serde::Serialize;

use super::{
    payment_intents::types::StripePaymentIntentResponse, refunds::types::StripeRefundResponse,
};
use crate::{
    core::{errors, webhooks::types::OutgoingWebhookType},
    headers,
    services::request::Maskable,
};

#[derive(Serialize, Debug)]
pub struct StripeOutgoingWebhook {
    id: String,
    #[serde(rename = "type")]
    stype: &'static str,
    object: &'static str,
    data: StripeWebhookObject,
    created: u64,
    // api_version: "2019-11-05", // not used
}

impl OutgoingWebhookType for StripeOutgoingWebhook {
    fn get_outgoing_webhooks_signature(
        &self,
        payment_response_hash_key: Option<String>,
    ) -> errors::CustomResult<Option<String>, errors::WebhooksFlowError> {
        let timestamp = self.created;

        let payment_response_hash_key = payment_response_hash_key
            .ok_or(errors::WebhooksFlowError::MerchantConfigNotFound)
            .into_report()
            .attach_printable("For stripe compatibility payment_response_hash_key is mandatory")?;

        let webhook_signature_payload =
            ext_traits::Encode::<serde_json::Value>::encode_to_string_of_json(self)
                .change_context(errors::WebhooksFlowError::OutgoingWebhookEncodingFailed)
                .attach_printable("failed encoding outgoing webhook payload")?;

        let new_signature_payload = format!("{timestamp}.{webhook_signature_payload}");
        let v1 = hex::encode(
            common_utils::crypto::HmacSha256::sign_message(
                &common_utils::crypto::HmacSha256,
                payment_response_hash_key.as_bytes(),
                new_signature_payload.as_bytes(),
            )
            .change_context(errors::WebhooksFlowError::OutgoingWebhookSigningFailed)
            .attach_printable("Failed to sign the message")?,
        );

        let t = timestamp;
        Ok(Some(format!("t={t},v1={v1}")))
    }

    fn add_webhook_header(header: &mut Vec<(String, Maskable<String>)>, signature: String) {
        header.push((
            headers::STRIPE_COMPATIBLE_WEBHOOK_SIGNATURE.to_string(),
            signature.into(),
        ))
    }
}

#[derive(Serialize, Debug)]
#[serde(tag = "type", content = "object", rename_all = "snake_case")]
pub enum StripeWebhookObject {
    PaymentIntent(StripePaymentIntentResponse),
    Refund(StripeRefundResponse),
    Dispute(StripeDisputeResponse),
    Mandate(StripeMandateResponse),
}

#[derive(Serialize, Debug)]
pub struct StripeDisputeResponse {
    pub id: String,
    pub amount: String,
    pub currency: String,
    pub payment_intent: String,
    pub reason: Option<String>,
    pub status: StripeDisputeStatus,
}

#[derive(Serialize, Debug)]
pub struct StripeMandateResponse {
    pub mandate_id: String,
    pub status: StripeMandateStatus,
    pub payment_method_id: String,
    pub payment_method: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StripeMandateStatus {
    Active,
    Inactive,
    Pending,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StripeDisputeStatus {
    WarningNeedsResponse,
    WarningUnderReview,
    WarningClosed,
    NeedsResponse,
    UnderReview,
    ChargeRefunded,
    Won,
    Lost,
}

impl From<api_models::disputes::DisputeResponse> for StripeDisputeResponse {
    fn from(res: api_models::disputes::DisputeResponse) -> Self {
        Self {
            id: res.dispute_id,
            amount: res.amount,
            currency: res.currency,
            payment_intent: res.payment_id,
            reason: res.connector_reason,
            status: StripeDisputeStatus::from(res.dispute_status),
        }
    }
}

impl From<api_models::mandates::MandateResponse> for StripeMandateResponse {
    fn from(res: api_models::mandates::MandateResponse) -> Self {
        Self {
            mandate_id: res.mandate_id,
            payment_method: res.payment_method,
            payment_method_id: res.payment_method_id,
            status: StripeMandateStatus::from(res.status),
        }
    }
}

impl From<MandateStatus> for StripeMandateStatus {
    fn from(status: MandateStatus) -> Self {
        match status {
            MandateStatus::Active => Self::Active,
            MandateStatus::Inactive | MandateStatus::Revoked => Self::Inactive,
            MandateStatus::Pending => Self::Pending,
        }
    }
}

impl From<DisputeStatus> for StripeDisputeStatus {
    fn from(status: DisputeStatus) -> Self {
        match status {
            DisputeStatus::DisputeOpened => Self::WarningNeedsResponse,
            DisputeStatus::DisputeExpired => Self::Lost,
            DisputeStatus::DisputeAccepted => Self::Lost,
            DisputeStatus::DisputeCancelled => Self::WarningClosed,
            DisputeStatus::DisputeChallenged => Self::WarningUnderReview,
            DisputeStatus::DisputeWon => Self::Won,
            DisputeStatus::DisputeLost => Self::Lost,
        }
    }
}

fn get_stripe_event_type(event_type: api_models::enums::EventType) -> &'static str {
    match event_type {
        api_models::enums::EventType::PaymentSucceeded => "payment_intent.succeeded",
        api_models::enums::EventType::PaymentFailed => "payment_intent.payment_failed",
        api_models::enums::EventType::PaymentProcessing => "payment_intent.processing",
        api_models::enums::EventType::PaymentCancelled => "payment_intent.canceled",

        // the below are not really stripe compatible because stripe doesn't provide this
        api_models::enums::EventType::ActionRequired => "action.required",
        api_models::enums::EventType::RefundSucceeded => "refund.succeeded",
        api_models::enums::EventType::RefundFailed => "refund.failed",
        api_models::enums::EventType::DisputeOpened => "dispute.failed",
        api_models::enums::EventType::DisputeExpired => "dispute.expired",
        api_models::enums::EventType::DisputeAccepted => "dispute.accepted",
        api_models::enums::EventType::DisputeCancelled => "dispute.cancelled",
        api_models::enums::EventType::DisputeChallenged => "dispute.challenged",
        api_models::enums::EventType::DisputeWon => "dispute.won",
        api_models::enums::EventType::DisputeLost => "dispute.lost",
        api_models::enums::EventType::MandateActive => "mandate.active",
        api_models::enums::EventType::MandateRevoked => "mandate.revoked",
    }
}

impl From<api::OutgoingWebhook> for StripeOutgoingWebhook {
    fn from(value: api::OutgoingWebhook) -> Self {
        Self {
            id: value.event_id,
            stype: get_stripe_event_type(value.event_type),
            data: StripeWebhookObject::from(value.content),
            object: "event",
            // put this conversion it into a function
            created: u64::try_from(value.timestamp.assume_utc().unix_timestamp()).unwrap_or_else(
                |error| {
                    logger::error!(
                        %error,
                        "incorrect value for `webhook.timestamp` provided {}", value.timestamp
                    );
                    // Current timestamp converted to Unix timestamp should have a positive value
                    // for many years to come
                    u64::try_from(date_time::now().assume_utc().unix_timestamp())
                        .unwrap_or_default()
                },
            ),
        }
    }
}

impl From<api::OutgoingWebhookContent> for StripeWebhookObject {
    fn from(value: api::OutgoingWebhookContent) -> Self {
        match value {
            api::OutgoingWebhookContent::PaymentDetails(payment) => {
                Self::PaymentIntent(payment.into())
            }
            api::OutgoingWebhookContent::RefundDetails(refund) => Self::Refund(refund.into()),
            api::OutgoingWebhookContent::DisputeDetails(dispute) => {
                Self::Dispute((*dispute).into())
            }
            api::OutgoingWebhookContent::MandateDetails(mandate) => {
                Self::Mandate((*mandate).into())
            }
        }
    }
}
