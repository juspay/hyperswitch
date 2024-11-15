#[cfg(feature = "payouts")]
use api_models::payouts as payout_models;
use api_models::{
    enums::{Currency, DisputeStatus, MandateStatus},
    webhooks::{self as api},
};
#[cfg(feature = "payouts")]
use common_utils::pii::{self, Email};
use common_utils::{crypto::SignMessage, date_time, ext_traits::Encode};
use error_stack::ResultExt;
use router_env::logger;
use serde::Serialize;

use super::{
    payment_intents::types::StripePaymentIntentResponse, refunds::types::StripeRefundResponse,
};
use crate::{
    core::{
        errors,
        webhooks::types::{OutgoingWebhookPayloadWithSignature, OutgoingWebhookType},
    },
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
        payment_response_hash_key: Option<impl AsRef<[u8]>>,
    ) -> errors::CustomResult<OutgoingWebhookPayloadWithSignature, errors::WebhooksFlowError> {
        let timestamp = self.created;

        let payment_response_hash_key = payment_response_hash_key
            .ok_or(errors::WebhooksFlowError::MerchantConfigNotFound)
            .attach_printable("For stripe compatibility payment_response_hash_key is mandatory")?;

        let webhook_signature_payload = self
            .encode_to_string_of_json()
            .change_context(errors::WebhooksFlowError::OutgoingWebhookEncodingFailed)
            .attach_printable("failed encoding outgoing webhook payload")?;

        let new_signature_payload = format!("{timestamp}.{webhook_signature_payload}");
        let v1 = hex::encode(
            common_utils::crypto::HmacSha256::sign_message(
                &common_utils::crypto::HmacSha256,
                payment_response_hash_key.as_ref(),
                new_signature_payload.as_bytes(),
            )
            .change_context(errors::WebhooksFlowError::OutgoingWebhookSigningFailed)
            .attach_printable("Failed to sign the message")?,
        );

        let t = timestamp;
        let signature = Some(format!("t={t},v1={v1}"));

        Ok(OutgoingWebhookPayloadWithSignature {
            payload: webhook_signature_payload.into(),
            signature,
        })
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
    PaymentIntent(Box<StripePaymentIntentResponse>),
    Refund(StripeRefundResponse),
    Dispute(StripeDisputeResponse),
    Mandate(StripeMandateResponse),
    #[cfg(feature = "payouts")]
    Payout(StripePayoutResponse),
}

#[derive(Serialize, Debug)]
pub struct StripeDisputeResponse {
    pub id: String,
    pub amount: String,
    pub currency: Currency,
    pub payment_intent: common_utils::id_type::PaymentId,
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

#[cfg(feature = "payouts")]
#[derive(Clone, Serialize, Debug)]
pub struct StripePayoutResponse {
    pub id: String,
    pub amount: i64,
    pub currency: String,
    pub payout_type: Option<common_enums::PayoutType>,
    pub status: StripePayoutStatus,
    pub name: Option<masking::Secret<String>>,
    pub email: Option<Email>,
    pub phone: Option<masking::Secret<String>>,
    pub phone_country_code: Option<String>,
    pub created: Option<i64>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub entity_type: common_enums::PayoutEntityType,
    pub recurring: bool,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
}

#[cfg(feature = "payouts")]
#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StripePayoutStatus {
    PayoutSuccess,
    PayoutFailure,
    PayoutProcessing,
    PayoutCancelled,
    PayoutInitiated,
    PayoutExpired,
    PayoutReversed,
}

#[cfg(feature = "payouts")]
impl From<common_enums::PayoutStatus> for StripePayoutStatus {
    fn from(status: common_enums::PayoutStatus) -> Self {
        match status {
            common_enums::PayoutStatus::Success => Self::PayoutSuccess,
            common_enums::PayoutStatus::Failed => Self::PayoutFailure,
            common_enums::PayoutStatus::Cancelled => Self::PayoutCancelled,
            common_enums::PayoutStatus::Initiated => Self::PayoutInitiated,
            common_enums::PayoutStatus::Expired => Self::PayoutExpired,
            common_enums::PayoutStatus::Reversed => Self::PayoutReversed,
            common_enums::PayoutStatus::Pending
            | common_enums::PayoutStatus::Ineligible
            | common_enums::PayoutStatus::RequiresCreation
            | common_enums::PayoutStatus::RequiresFulfillment
            | common_enums::PayoutStatus::RequiresPayoutMethodData
            | common_enums::PayoutStatus::RequiresVendorAccountCreation
            | common_enums::PayoutStatus::RequiresConfirmation => Self::PayoutProcessing,
        }
    }
}

#[cfg(feature = "payouts")]
impl From<payout_models::PayoutCreateResponse> for StripePayoutResponse {
    fn from(res: payout_models::PayoutCreateResponse) -> Self {
        let (name, email, phone, phone_country_code) = match res.customer {
            Some(customer) => (
                customer.name,
                customer.email,
                customer.phone,
                customer.phone_country_code,
            ),
            None => (None, None, None, None),
        };
        Self {
            id: res.payout_id,
            amount: res.amount.get_amount_as_i64(),
            currency: res.currency.to_string(),
            payout_type: res.payout_type,
            status: StripePayoutStatus::from(res.status),
            name,
            email,
            phone,
            phone_country_code,
            created: res.created.map(|t| t.assume_utc().unix_timestamp()),
            metadata: res.metadata,
            entity_type: res.entity_type,
            recurring: res.recurring,
            error_message: res.error_message,
            error_code: res.error_code,
        }
    }
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

        // as per this doc https://stripe.com/docs/api/events/types#event_types-payment_intent.amount_capturable_updated
        api_models::enums::EventType::PaymentAuthorized => {
            "payment_intent.amount_capturable_updated"
        }
        // stripe treats partially captured payments as succeeded.
        api_models::enums::EventType::PaymentCaptured => "payment_intent.succeeded",
        api_models::enums::EventType::PayoutSuccess => "payout.paid",
        api_models::enums::EventType::PayoutFailed => "payout.failed",
        api_models::enums::EventType::PayoutInitiated => "payout.created",
        api_models::enums::EventType::PayoutCancelled => "payout.canceled",
        api_models::enums::EventType::PayoutProcessing => "payout.created",
        api_models::enums::EventType::PayoutExpired => "payout.failed",
        api_models::enums::EventType::PayoutReversed => "payout.reconciliation_completed",
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
                Self::PaymentIntent(Box::new((*payment).into()))
            }
            api::OutgoingWebhookContent::RefundDetails(refund) => Self::Refund((*refund).into()),
            api::OutgoingWebhookContent::DisputeDetails(dispute) => {
                Self::Dispute((*dispute).into())
            }
            api::OutgoingWebhookContent::MandateDetails(mandate) => {
                Self::Mandate((*mandate).into())
            }
            #[cfg(feature = "payouts")]
            api::OutgoingWebhookContent::PayoutDetails(payout) => Self::Payout((*payout).into()),
        }
    }
}
