use error_stack::ResultExt;
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use super::requests::*;

#[derive(Clone, Debug, PartialEq, Serialize)] // Remove Deserialize here
#[serde(rename_all = "camelCase")]
pub struct WorldpayPaymentsResponse {
    pub outcome: PaymentOutcome,
    pub transaction_reference: Option<String>,
    #[serde(flatten)]
    pub other_fields: Option<WorldpayPaymentResponseFields>,
}

// Implement a custom deserializer for WorldpayPaymentsResponse
impl<'de> Deserialize<'de> for WorldpayPaymentsResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        use serde::de::MapAccess;
        use serde::de::Visitor;
        use std::fmt;

        enum Field {
            Outcome,
            TransactionReference,
            OtherFields,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`outcome`, `transactionReference`, or other payment response fields")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "outcome" => Ok(Field::Outcome),
                            "transactionReference" => Ok(Field::TransactionReference),
                            "outcome" | "transactionReference" | "paymentInstrument" | "issuer" | "scheme" | "_links" | "_actions" | "description" | "riskFactors" | "fraud" | "token" | "schemeReference" | "refusalDescription" | "refusalCode" | "threeDS" | "advice" | "authentication" | "challenge" | "deviceDataCollection" => Ok(Field::OtherFields),
                            _ => Err(E::unknown_field(value, &[])),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct WorldpayPaymentsResponseVisitor;

        impl<'de> Visitor<'de> for WorldpayPaymentsResponseVisitor {
            type Value = WorldpayPaymentsResponse;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct WorldpayPaymentsResponse")
            }

            fn visit_map<V>(self, mut map: V) -> Result<WorldpayPaymentsResponse, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut outcome = None;
                let mut transaction_reference = None;
                let mut other_fields_map: serde_json::Map<String, serde_json::Value> =
                    serde_json::Map::new();

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "outcome" => {
                            if outcome.is_some() {
                                return Err(V::Error::duplicate_field("outcome"));
                            }
                            outcome = Some(map.next_value()?);
                        }
                        "transactionReference" => {
                            if transaction_reference.is_some() {
                                return Err(V::Error::duplicate_field("transactionReference"));
                            }
                            transaction_reference = Some(map.next_value()?);
                        }
                        _ => {
                            // Collect all other fields into a map
                            let value = map.next_value()?;
                            other_fields_map.insert(key, value);
                        }
                    }
                }

                let outcome: PaymentOutcome =
                    outcome.ok_or_else(|| V::Error::missing_field("outcome"))?;

                let other_fields = match outcome {
                    PaymentOutcome::Authorized => {
                        let auth_res: AuthorizedResponse =
                            serde_json::from_value(serde_json::Value::Object(other_fields_map))
                                .map_err(V::Error::custom)?;
                        Some(WorldpayPaymentResponseFields::AuthorizedResponse(Box::new(
                            auth_res,
                        )))
                    }
                    PaymentOutcome::Refused => {
                        let refused_res: RefusedResponse =
                            serde_json::from_value(serde_json::Value::Object(other_fields_map))
                                .map_err(V::Error::custom)?;
                        Some(WorldpayPaymentResponseFields::RefusedResponse(refused_res))
                    }
                    PaymentOutcome::ThreeDsDeviceDataRequired => {
                        let ddc_res: DDCResponse =
                            serde_json::from_value(serde_json::Value::Object(other_fields_map))
                                .map_err(V::Error::custom)?;
                        Some(WorldpayPaymentResponseFields::DDCResponse(ddc_res))
                    }
                    PaymentOutcome::ThreeDsChallenged => {
                        let three_ds_challenged_res: ThreeDsChallengedResponse =
                            serde_json::from_value(serde_json::Value::Object(other_fields_map))
                                .map_err(V::Error::custom)?;
                        Some(WorldpayPaymentResponseFields::ThreeDsChallenged(
                            three_ds_challenged_res,
                        ))
                    }
                    PaymentOutcome::FraudHighRisk => {
                        let fraud_high_risk_res: FraudHighRiskResponse =
                            serde_json::from_value(serde_json::Value::Object(other_fields_map))
                                .map_err(V::Error::custom)?;
                        Some(WorldpayPaymentResponseFields::FraudHighRisk(
                            fraud_high_risk_res,
                        ))
                    }
                    _ => None, // For other outcomes, there might not be specific `other_fields` or they are not currently handled.
                };

                Ok(WorldpayPaymentsResponse {
                    outcome,
                    transaction_reference,
                    other_fields,
                })
            }
        }

        deserializer.deserialize_map(WorldpayPaymentsResponseVisitor)
    }
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
    pub payment_instrument: PaymentsResPaymentInstrument,
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
    /// Mandate's token
    pub token: Option<MandateToken>,
    /// Network transaction ID
    pub scheme_reference: Option<Secret<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MandateToken {
    pub href: Secret<String>,
    pub token_id: String,
    pub token_expiry_date_time: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FraudHighRiskResponse {
    pub score: f32,
    pub reason: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefusedResponse {
    // These fields should remain non-optional for the untagged enum to work correctly
    // IF we keep the custom deserializer in WorldpayPaymentsResponse
    // Otherwise, if we rely solely on `untagged`, they would need to be optional
    // for RefusedResponse to be a fallback, which is the problem Kashif-m highlighted.
    // With the custom deserializer, we explicitly parse based on 'outcome'.
    pub refusal_description: String,
    pub refusal_code: String,
    pub risk_factors: Option<Vec<RiskFactorsInner>>,
    pub fraud: Option<Fraud>,
    #[serde(rename = "threeDS")]
    pub three_ds: Option<ThreeDsResponse>,
    pub advice: Option<Advice>,
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Advice {
    pub code: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDsResponse {
    pub outcome: String,
    pub issuer_response: IssuerResponse,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDsChallengedResponse {
    pub authentication: AuthenticationResponse,
    pub challenge: ThreeDsChallenge,
    #[serde(rename = "_actions")]
    pub actions: CompleteThreeDsActionLink,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuthenticationResponse {
    pub version: String,
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
#[serde(rename_all = "lowercase")]
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
#[serde(rename_all = "camelCase")]
pub struct ActionLinks {
    supply_3ds_device_data: Option<ActionLink>,
    settle_payment: Option<ActionLink>,
    partially_settle_payment: Option<ActionLink>,
    refund_payment: Option<ActionLink>,
    partially_refund_payment: Option<ActionLink>,
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
    let optional_reference_id = response
        .other_fields
        .as_ref()
        .and_then(|other_fields| match other_fields {
            WorldpayPaymentResponseFields::AuthorizedResponse(res) => res
                .links
                .as_ref()
                .and_then(|link| {
                    link.self_link
                        .href
                        .rsplit_once('/')
                        .map(|(_, h)| h.to_string())
                }),
            WorldpayPaymentResponseFields::DDCResponse(res) => res
                .actions
                .supply_ddc_data
                .href
                .split('/')
                .nth_back(1)
                .map(|s| s.to_string()),
            WorldpayPaymentResponseFields::ThreeDsChallenged(res) => res
                .actions
                .complete_three_ds_challenge
                .href
                .split('/')
                .nth_back(1)
                .map(|s| s.to_string()),
            WorldpayPaymentResponseFields::FraudHighRisk(_) => None,
            WorldpayPaymentResponseFields::RefusedResponse(res) => {
                let refusal_code = res
                    .refusal_code
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(crate::utils::NO_ERROR_CODE); // Assuming crate::utils::NO_ERROR_CODE exists

                let refusal_description = res
                    .refusal_description
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(crate::utils::NO_ERROR_MESSAGE); // Assuming crate::utils::NO_ERROR_MESSAGE exists

                tracing::warn!(
                    error_code = refusal_code,
                    error_message = refusal_description,
                    "Received a refused response with possibly missing error fields"
                );
                None
            }
        })
        .map(|href| {
            urlencoding::decode(&href)
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
    #[serde(rename = "type")]
    pub payment_instrument_type: String,
    pub card_bin: Option<String>,
    pub last_four: Option<String>,
    pub expiry_date: Option<ExpiryDate>,
    pub card_brand: Option<String>,
    pub funding_type: Option<String>,
    pub category: Option<String>,
    pub issuer_name: Option<String>,
    pub payment_account_reference: Option<String>,
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

/// Worldpay's unique reference ID for a request
pub const WP_CORRELATION_ID: &str = "WP-CorrelationId";



