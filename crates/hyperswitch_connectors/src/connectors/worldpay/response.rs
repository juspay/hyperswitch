use error_stack::ResultExt;
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use super::requests::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayPaymentsResponse {
    pub outcome: PaymentOutcome,
    pub transaction_reference: Option<String>,
    #[serde(flatten)]
    pub other_fields: Option<WorldpayPaymentResponseFields>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorldpayPaymentResponseFields {
    RefusedResponse(RefusedResponse),
    DDCResponse(DDCResponse),
    ThreeDsChallenged(ThreeDsChallengedResponse),
    FraudHighRisk(FraudHighRiskResponse),
    AuthorizedResponse(Box<AuthorizedResponse>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_instrument: Option<Secret<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<Secret<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<Secret<serde_json::Value>>,
    #[serde(rename = "_links", skip_serializing_if = "Option::is_none")]
    pub links: Option<SelfLink>,
    #[serde(rename = "_actions", skip_serializing_if = "Option::is_none")]
    pub actions: Option<Secret<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_factors: Option<Secret<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fraud: Option<Secret<serde_json::Value>>,
    /// Mandate's token
    pub token: Option<MandateToken>,
    /// Network transaction ID
    pub scheme_reference: Option<Secret<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FraudHighRiskResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Secret<serde_json::Value>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefusedResponse {
    pub refusal_description: String,
    pub refusal_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_factors: Option<Secret<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fraud: Option<Secret<serde_json::Value>>,
    #[serde(rename = "threeDS", skip_serializing_if = "Option::is_none")]
    pub three_ds: Option<Secret<serde_json::Value>>,
    pub advice: Option<Advice>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Advice {
    pub code: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDsChallengedResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<Secret<serde_json::Value>>,
    pub challenge: ThreeDsChallenge,
    #[serde(rename = "_actions")]
    pub actions: CompleteThreeDsActionLink,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThreeDsChallenge {
    pub reference: String,
    pub url: Url,
    pub jwt: Secret<String>,
    pub payload: Secret<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompleteThreeDsActionLink {
    #[serde(rename = "complete3dsChallenge")]
    pub complete_three_ds_challenge: ActionLink,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DDCResponse {
    pub device_data_collection: DDCToken,
    #[serde(rename = "_actions")]
    pub actions: DDCActionLink,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DDCToken {
    pub jwt: Secret<String>,
    pub url: Url,
    pub bin: Secret<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DDCActionLink {
    #[serde(rename = "supply3dsDeviceData")]
    supply_ddc_data: ActionLink,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentOutcome {
    #[serde(alias = "authorized", alias = "Authorized")]
    Authorized,
    Refused,
    SentForSettlement,
    SentForRefund,
    FraudHighRisk,
    #[serde(alias = "3dsDeviceDataRequired")]
    ThreeDsDeviceDataRequired,
    SentForCancellation,
    #[serde(alias = "3dsAuthenticationFailed")]
    ThreeDsAuthenticationFailed,
    SentForPartialRefund,
    #[serde(alias = "3dsChallenged")]
    ThreeDsChallenged,
    #[serde(alias = "3dsUnavailable")]
    ThreeDsUnavailable,
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for PaymentOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Authorized => write!(f, "authorized"),
            Self::Refused => write!(f, "refused"),
            Self::SentForSettlement => write!(f, "sentForSettlement"),
            Self::SentForRefund => write!(f, "sentForRefund"),
            Self::FraudHighRisk => write!(f, "fraudHighRisk"),
            Self::ThreeDsDeviceDataRequired => write!(f, "3dsDeviceDataRequired"),
            Self::SentForCancellation => write!(f, "sentForCancellation"),
            Self::ThreeDsAuthenticationFailed => write!(f, "3dsAuthenticationFailed"),
            Self::SentForPartialRefund => write!(f, "sentForPartialRefund"),
            Self::ThreeDsChallenged => write!(f, "3dsChallenged"),
            Self::ThreeDsUnavailable => write!(f, "3dsUnavailable"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelfLink {
    #[serde(rename = "self")]
    pub self_link: SelfLinkInner,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelfLinkInner {
    pub href: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionLink {
    pub href: String,
    pub method: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayEventResponse {
    pub last_event: EventType,
    #[serde(rename = "_links", skip_serializing_if = "Option::is_none")]
    pub links: Option<EventLinks>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventType {
    SentForAuthorization,
    #[serde(alias = "Authorized")]
    Authorized,
    #[serde(alias = "Sent for Settlement")]
    SentForSettlement,
    Settled,
    SettlementFailed,
    Cancelled,
    Error,
    Expired,
    Refused,
    #[serde(alias = "Sent for Refund")]
    SentForRefund,
    Refunded,
    RefundFailed,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct EventLinks {
    #[serde(rename = "payments:events", skip_serializing_if = "Option::is_none")]
    pub events: Option<String>,
}

pub fn get_resource_id<T, F>(
    response: WorldpayPaymentsResponse,
    connector_transaction_id: Option<String>,
    transform_fn: F,
) -> Result<T, error_stack::Report<errors::ConnectorError>>
where
    F: Fn(String) -> T,
{
    let optional_reference_id = response
        .other_fields
        .as_ref()
        .and_then(|other_fields| match other_fields {
            WorldpayPaymentResponseFields::AuthorizedResponse(res) => res
                .links
                .as_ref()
                .and_then(|link| link.self_link.href.rsplit_once('/').map(|(_, h)| h)),
            WorldpayPaymentResponseFields::DDCResponse(res) => {
                res.actions.supply_ddc_data.href.split('/').nth_back(1)
            }
            WorldpayPaymentResponseFields::ThreeDsChallenged(res) => res
                .actions
                .complete_three_ds_challenge
                .href
                .split('/')
                .nth_back(1),
            WorldpayPaymentResponseFields::FraudHighRisk(_)
            | WorldpayPaymentResponseFields::RefusedResponse(_) => None,
        })
        .map(|href| {
            urlencoding::decode(href)
                .map(|s| transform_fn(s.into_owned()))
                .change_context(errors::ConnectorError::ResponseHandlingFailed)
        })
        .transpose()?;
    optional_reference_id
        .or_else(|| connector_transaction_id.map(transform_fn))
        .ok_or_else(|| {
            errors::ConnectorError::MissingRequiredField {
                field_name: "_links.self.href",
            }
            .into()
        })
}

pub struct ResponseIdStr {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MandateToken {
    pub href: Secret<String>,
    pub token_id: String,
    pub token_expiry_date_time: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayErrorResponse {
    pub error_name: String,
    pub message: String,
    pub validation_errors: Option<serde_json::Value>,
}

impl WorldpayErrorResponse {
    pub fn default(status_code: u16) -> Self {
        match status_code {
            code @ 404 => Self {
                error_name: format!("{code} Not found"),
                message: "Resource not found".to_string(),
                validation_errors: None,
            },
            code => Self {
                error_name: code.to_string(),
                message: "Unknown error".to_string(),
                validation_errors: None,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayWebhookTransactionId {
    pub event_details: EventDetails,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDetails {
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub transaction_reference: String,
    /// Mandate's token
    pub token: Option<MandateToken>,
    /// Network transaction ID
    pub scheme_reference: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayWebhookEventType {
    pub event_id: String,
    pub event_timestamp: String,
    pub event_details: EventDetails,
}

/// Worldpay's unique reference ID for a request
pub(super) const WP_CORRELATION_ID: &str = "WP-CorrelationId";
