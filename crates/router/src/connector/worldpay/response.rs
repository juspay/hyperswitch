use masking::Secret;
use serde::{Deserialize, Serialize};

use super::requests::*;
use crate::{core::errors, types, types::transformers::ForeignTryFrom};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayPaymentsResponse {
    pub outcome: Option<PaymentOutcome>,
    /// Any risk factors which have been identified for the authorization. This section will not appear if no risks are identified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_factors: Option<Vec<RiskFactorsInner>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<Issuer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<PaymentsResponseScheme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_instrument: Option<PaymentsResPaymentInstrument>,
    #[serde(rename = "_links", skip_serializing_if = "Option::is_none")]
    pub links: Option<PaymentLinks>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentOutcome {
    #[serde(alias = "authorized", alias = "Authorized")]
    Authorized,
    Refused,
    #[serde(alias = "Sent for Settlement")]
    SentForSettlement,
    #[serde(alias = "Sent for Refund")]
    SentForRefund,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RefundOutcome {
    #[serde(alias = "Sent for Refund")]
    SentForRefund,
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
pub struct PaymentLinks {
    #[serde(
        rename = "cardPayments:events",
        skip_serializing_if = "Option::is_none"
    )]
    pub events: Option<PaymentLink>,
    #[serde(
        rename = "cardPayments:settle",
        skip_serializing_if = "Option::is_none"
    )]
    pub settle_event: Option<PaymentLink>,
    #[serde(
        rename = "cardPayments:partialSettle",
        skip_serializing_if = "Option::is_none"
    )]
    pub partial_settle_event: Option<PaymentLink>,
    #[serde(
        rename = "cardPayments:refund",
        skip_serializing_if = "Option::is_none"
    )]
    pub refund_event: Option<PaymentLink>,
    #[serde(
        rename = "cardPayments:partialRefund",
        skip_serializing_if = "Option::is_none"
    )]
    pub partial_refund_event: Option<PaymentLink>,
    #[serde(
        rename = "cardPayments:reverse",
        skip_serializing_if = "Option::is_none"
    )]
    pub reverse_event: Option<PaymentLink>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct EventLinks {
    #[serde(rename = "payments:events", skip_serializing_if = "Option::is_none")]
    pub events: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PaymentLink {
    pub href: String,
}

fn get_resource_id<T, F>(
    links: Option<PaymentLinks>,
    transform_fn: F,
) -> Result<T, error_stack::Report<errors::ConnectorError>>
where
    F: Fn(String) -> T,
{
    let reference_id = links
        .and_then(|l| l.events)
        .and_then(|e| e.href.rsplit_once('/').map(|h| h.1.to_string()))
        .map(transform_fn);
    reference_id.ok_or_else(|| {
        errors::ConnectorError::MissingRequiredField {
            field_name: "links.events",
        }
        .into()
    })
}

pub struct ResponseIdStr {
    pub id: String,
}

impl TryFrom<Option<PaymentLinks>> for ResponseIdStr {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(links: Option<PaymentLinks>) -> Result<Self, Self::Error> {
        get_resource_id(links, |id| Self { id })
    }
}

impl ForeignTryFrom<Option<PaymentLinks>> for types::ResponseId {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(links: Option<PaymentLinks>) -> Result<Self, Self::Error> {
        get_resource_id(links, Self::ConnectorTransactionId)
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issuer {
    pub authorization_code: Secret<String>,
}

impl Issuer {
    pub fn new(code: String) -> Self {
        Self {
            authorization_code: Secret::new(code),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsResPaymentInstrument {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub payment_instrument_type: Option<String>,
    pub card_bin: Option<String>,
    pub last_four: Option<String>,
    pub category: Option<String>,
    pub expiry_date: Option<ExpiryDate>,
    pub card_brand: Option<String>,
    pub funding_type: Option<String>,
    pub issuer_name: Option<String>,
    pub payment_account_reference: Option<String>,
}

impl PaymentsResPaymentInstrument {
    pub fn new() -> Self {
        Self {
            payment_instrument_type: None,
            card_bin: None,
            last_four: None,
            category: None,
            expiry_date: None,
            card_brand: None,
            funding_type: None,
            issuer_name: None,
            payment_account_reference: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskFactorsInner {
    #[serde(rename = "type")]
    pub risk_type: RiskType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<Detail>,
    pub risk: Risk,
}

impl RiskFactorsInner {
    pub fn new(risk_type: RiskType, risk: Risk) -> Self {
        Self {
            risk_type,
            detail: None,
            risk,
        }
    }
}

#[derive(
    Clone, Copy, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum RiskType {
    #[default]
    Avs,
    Cvc,
    RiskProfile,
}

#[derive(
    Clone, Copy, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum Detail {
    #[default]
    Address,
    Postcode,
}

#[derive(
    Clone, Copy, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum Risk {
    #[default]
    NotChecked,
    NotMatched,
    NotSupplied,
    VerificationFailed,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PaymentsResponseScheme {
    pub reference: String,
}

impl PaymentsResponseScheme {
    pub fn new(reference: String) -> Self {
        Self { reference }
    }
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
                error_name: format!("{} Not found", code),
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
    pub transaction_reference: String,
    #[serde(rename = "type")]
    pub event_type: EventType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayWebhookEventType {
    pub event_id: String,
    pub event_timestamp: String,
    pub event_details: EventDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum WorldpayWebhookStatus {
    SentForSettlement,
    Authorized,
    SentForAuthorization,
    Cancelled,
    Error,
    Expired,
    Refused,
    SentForRefund,
    RefundFailed,
}
