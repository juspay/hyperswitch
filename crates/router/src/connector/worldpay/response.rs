use serde::{Deserialize, Serialize};

use crate::{core::errors, types};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exemption: Option<Box<Exemption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<Box<Issuer>>,
    pub outcome: Option<Outcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_instrument: Option<Box<PaymentsResPaymentInstrument>>,
    /// Any risk factors which have been identified for the authorization. This section will not appear if no risks are identified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_factors: Option<Vec<RiskFactorsInner>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<Box<PaymentsResponseScheme>>,
    #[serde(rename = "_links", skip_serializing_if = "Option::is_none")]
    pub _links: Option<PaymentLinks>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Authorized,
    Refused,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventResponse {
    pub last_event: EventType,
    #[serde(rename = "_links", skip_serializing_if = "Option::is_none")]
    pub _links: Option<EventLinks>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventType {
    Authorized,
    Cancelled,
    Charged,
    SentForRefund,
    RefundFailed,
    Refused,
    Refunded,
    Error,
    CaptureFailed,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Exemption {
    pub result: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PaymentLinks {
    #[serde(rename = "payments:events", skip_serializing_if = "Option::is_none")]
    pub events: Option<PaymentLink>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct EventLinks {
    #[serde(rename = "payments:events", skip_serializing_if = "Option::is_none")]
    pub events: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PaymentLink {
    #[serde(rename = "href")]
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
    match reference_id {
        Some(response_id) => Ok(response_id),
        None => Err(errors::ConnectorError::MissingRequiredField {
            field_name: "links.events".to_string(),
        })?,
    }
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

impl TryFrom<Option<PaymentLinks>> for types::ResponseId {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(links: Option<PaymentLinks>) -> Result<Self, Self::Error> {
        get_resource_id(links, Self::ConnectorTransactionId)
    }
}

impl Exemption {
    pub fn new(result: String, reason: String) -> Self {
        Self { result, reason }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issuer {
    pub authorization_code: String,
}

impl Issuer {
    pub fn new(authorization_code: String) -> Self {
        Self { authorization_code }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PaymentsResPaymentInstrument {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub risk_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<Box<PaymentInstrumentCard>>,
}

impl PaymentsResPaymentInstrument {
    pub fn new() -> Self {
        Self {
            risk_type: None,
            card: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInstrumentCard {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<Box<PaymentInstrumentCardNumber>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<Box<PaymentInstrumentCardIssuer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_account_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub funding_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<Box<PaymentInstrumentCardExpiryDate>>,
}

impl PaymentInstrumentCard {
    pub fn new() -> Self {
        Self {
            number: None,
            issuer: None,
            payment_account_reference: None,
            country_code: None,
            funding_type: None,
            brand: None,
            expiry_date: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInstrumentCardExpiryDate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
}

impl PaymentInstrumentCardExpiryDate {
    pub fn new() -> Self {
        Self {
            month: None,
            year: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInstrumentCardIssuer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl PaymentInstrumentCardIssuer {
    pub fn new() -> Self {
        Self { name: None }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInstrumentCardNumber {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last4_digits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpan: Option<String>,
}

impl PaymentInstrumentCardNumber {
    pub fn new() -> Self {
        Self {
            bin: None,
            last4_digits: None,
            dpan: None,
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
pub enum Risk {
    #[default]
    #[serde(rename = "not_checked")]
    NotChecked,
    #[serde(rename = "not_matched")]
    NotMatched,
    #[serde(rename = "not_supplied")]
    NotSupplied,
    #[serde(rename = "verificationFailed")]
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
