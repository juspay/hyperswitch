use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use super::requests::*;
use crate::core::errors;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayPaymentsResponse {
    pub outcome: PaymentOutcome,
    pub transaction_reference: Option<String>,
    #[serde(flatten)]
    pub other_fields: WorldpayPaymentResponseFields,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorldpayPaymentResponseFields {
    AuthorizedResponse(Box<AuthorizedResponse>),
    DDCResponse(DDCResponse),
    FraudHighRisk(FraudHighRiskResponse),
    RefusedResponse(RefusedResponse),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_instrument: Option<PaymentsResPaymentInstrument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<Issuer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<PaymentsResponseScheme>,
    #[serde(rename = "_links", skip_serializing_if = "Option::is_none")]
    pub links: Option<SelfLink>,
    #[serde(rename = "_actions")]
    pub actions: Option<ActionLinks>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub risk_factors: Option<Vec<RiskFactorsInner>>,
    pub fraud: Option<Fraud>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FraudHighRiskResponse {
    pub score: f32,
    pub reason: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefusedResponse {
    pub refusal_description: String,
    pub refusal_code: String,
    pub risk_factors: Vec<RiskFactorsInner>,
    pub fraud: Fraud,
    #[serde(rename = "threeDS")]
    pub three_ds: Option<ThreeDsResponse>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDsResponse {
    pub outcome: String,
    pub issuer_response: IssuerResponse,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IssuerResponse {
    Challenged,
    Frictionless,
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
    pub jwt: String,
    pub url: String,
    pub bin: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DDCActionLink {
    #[serde(rename = "supply3dsDeviceData")]
    supply_ddc_data: ActionLink,
    method: String,
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
    ThreeDsChallenged,
    SentForCancellation,
    #[serde(alias = "3dsAuthenticationFailed")]
    ThreeDsAuthenticationFailed,
    SentForPartialRefund,
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
#[serde(rename_all = "camelCase")]
pub struct ActionLinks {
    supply_3ds_device_data: Option<ActionLink>,
    settle_payment: Option<ActionLink>,
    partially_settle_payment: Option<ActionLink>,
    refund_payment: Option<ActionLink>,
    partiall_refund_payment: Option<ActionLink>,
    cancel_payment: Option<ActionLink>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionLink {
    pub href: String,
    pub method: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fraud {
    pub outcome: FraudOutcome,
    pub score: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FraudOutcome {
    LowRisk,
    HighRisk,
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

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PaymentLink {
    pub href: String,
}

pub fn get_resource_id<T, F>(
    response: WorldpayPaymentsResponse,
    connector_transaction_id: Option<String>,
    transform_fn: F,
) -> Result<T, error_stack::Report<errors::ConnectorError>>
where
    F: Fn(String) -> T,
{
    let reference_id = match response.other_fields {
        WorldpayPaymentResponseFields::AuthorizedResponse(res) => res
            .links
            .as_ref()
            .and_then(|link| link.self_link.href.rsplit_once('/'))
            .map(|(_, h)| urlencoding::decode(h))
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?
            .map(|s| transform_fn(s.into_owned())),
        WorldpayPaymentResponseFields::DDCResponse(res) => res
            .actions
            .supply_ddc_data
            .href
            .split('/')
            .rev()
            .nth(1)
            .map(urlencoding::decode)
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?
            .map(|s| transform_fn(s.into_owned())),
        WorldpayPaymentResponseFields::FraudHighRisk(_) => None,
        WorldpayPaymentResponseFields::RefusedResponse(_) => None,
    };
    reference_id
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
    pub expiry_date: Option<ExpiryDate>,
    pub card_brand: Option<String>,
    pub funding_type: Option<String>,
    pub category: Option<String>,
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
#[serde(rename_all = "lowercase")]
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
