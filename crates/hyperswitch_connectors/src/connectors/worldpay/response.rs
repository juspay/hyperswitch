use error_stack::ResultExt;
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use super::requests::*;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayPaymentsResponse {
    pub outcome: PaymentOutcome,
    pub transaction_reference: Option<String>,
    #[serde(flatten)]
    pub other_fields: Option<WorldpayPaymentResponseFields>,
}

impl<'de> Deserialize<'de> for WorldpayPaymentsResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawWorldpayPaymentsResponse {
            outcome: PaymentOutcome,
            transaction_reference: Option<String>,
            #[serde(flatten)]
            other_fields: serde_json::Map<String, serde_json::Value>,
        }

        let raw = RawWorldpayPaymentsResponse::deserialize(deserializer)?;
        let other_fields = WorldpayPaymentResponseFields::from_outcome(
            &raw.outcome,
            serde_json::Value::Object(raw.other_fields),
        );

        Ok(Self {
            outcome: raw.outcome,
            transaction_reference: raw.transaction_reference,
            other_fields,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum WorldpayPaymentResponseFields {
    RefusedResponse(RefusedResponse),
    DDCResponse(DDCResponse),
    ThreeDsChallenged(ThreeDsChallengedResponse),
    FraudHighRisk(FraudHighRiskResponse),
    AuthorizedResponse(Box<AuthorizedResponse>),
}

impl WorldpayPaymentResponseFields {
    /// Picks the response shape from the payment `outcome` instead of trying each shape in turn,
    /// so a refusal missing `refusalCode` or `refusalDescription` is still read as a refusal.
    /// A body that does not match the shape of its outcome carries no extra fields, as before.
    fn from_outcome(outcome: &PaymentOutcome, fields: serde_json::Value) -> Option<Self> {
        match outcome {
            PaymentOutcome::Refused => parse_fields(outcome, fields).map(Self::RefusedResponse),
            PaymentOutcome::ThreeDsDeviceDataRequired => {
                parse_fields(outcome, fields).map(Self::DDCResponse)
            }
            PaymentOutcome::ThreeDsChallenged => {
                parse_fields(outcome, fields).map(Self::ThreeDsChallenged)
            }
            PaymentOutcome::FraudHighRisk => parse_fields(outcome, fields).map(Self::FraudHighRisk),
            PaymentOutcome::Authorized
            | PaymentOutcome::SentForSettlement
            | PaymentOutcome::SentForRefund
            | PaymentOutcome::SentForCancellation
            | PaymentOutcome::SentForPartialRefund
            | PaymentOutcome::ThreeDsAuthenticationFailed
            | PaymentOutcome::ThreeDsUnavailable => parse_fields(outcome, fields)
                .map(|response| Self::AuthorizedResponse(Box::new(response))),
        }
    }
}

/// Parses the fields accompanying `outcome`. A mismatch is logged and yields `None` rather than
/// failing the whole response: capture, void and refund responses legitimately carry only
/// `_links`, and a refusal must not turn into a parsing failure because of one unexpected value.
fn parse_fields<T: serde::de::DeserializeOwned>(
    outcome: &PaymentOutcome,
    fields: serde_json::Value,
) -> Option<T> {
    serde_json::from_value(fields)
        .map_err(|error| {
            router_env::logger::warn!(
                worldpay_outcome = %outcome,
                expected_fields = std::any::type_name::<T>(),
                error = deserialization_error_detail(&error),
                "Worldpay response fields do not match the shape expected for the outcome"
            );
        })
        .ok()
}

/// Describes a deserialization error without echoing response values, which serde includes in
/// messages such as `invalid type: string "..."` and which may be sensitive.
fn deserialization_error_detail(error: &serde_json::Error) -> String {
    let message = error.to_string();
    if message.starts_with("missing field") {
        message
    } else {
        format!("{:?} error", error.classify())
    }
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
    pub refusal_description: Option<String>,
    // Access Worldpay returns a raw response code in the refusalCode field (if enabled) containing the unmodified response code received either directly from the card scheme for Worldpay-acquired transactions, or from third party acquirers.
    pub refusal_code: Option<String>,
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
                field_name: "_links.self.href".into(),
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

/// Worldpay's unique reference ID for a request
pub(super) const WP_CORRELATION_ID: &str = "WP-CorrelationId";

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn parse(body: serde_json::Value) -> WorldpayPaymentsResponse {
        serde_json::from_value(body).unwrap()
    }

    fn refused_fields(response: WorldpayPaymentsResponse) -> RefusedResponse {
        match response.other_fields {
            Some(WorldpayPaymentResponseFields::RefusedResponse(res)) => res,
            other => panic!("expected RefusedResponse, got {other:?}"),
        }
    }

    #[test]
    fn refused_with_code_and_description_is_parsed_as_refused_response() {
        let response = parse(serde_json::json!({
            "outcome": "refused",
            "refusalCode": "5",
            "refusalDescription": "Refused",
            "advice": { "code": "03" },
            "riskFactors": [{ "type": "cvc", "risk": "notMatched" }]
        }));

        assert_eq!(response.outcome, PaymentOutcome::Refused);
        let refused = refused_fields(response);
        assert_eq!(refused.refusal_code.as_deref(), Some("5"));
        assert_eq!(refused.refusal_description.as_deref(), Some("Refused"));
        assert_eq!(
            refused.advice.and_then(|advice| advice.code).as_deref(),
            Some("03")
        );
    }

    #[test]
    fn refused_with_only_description_is_parsed_as_refused_response() {
        let refused = refused_fields(parse(serde_json::json!({
            "outcome": "refused",
            "refusalDescription": "Do not honour"
        })));

        assert_eq!(refused.refusal_code, None);
        assert_eq!(
            refused.refusal_description.as_deref(),
            Some("Do not honour")
        );
    }

    #[test]
    fn refused_with_only_code_is_parsed_as_refused_response() {
        let refused = refused_fields(parse(serde_json::json!({
            "outcome": "refused",
            "refusalCode": "51"
        })));

        assert_eq!(refused.refusal_code.as_deref(), Some("51"));
        assert_eq!(refused.refusal_description, None);
    }

    #[test]
    fn refused_without_code_or_description_is_parsed_as_refused_response() {
        let refused = refused_fields(parse(serde_json::json!({ "outcome": "refused" })));

        assert_eq!(refused.refusal_code, None);
        assert_eq!(refused.refusal_description, None);
    }

    #[test]
    fn authorized_is_parsed_as_authorized_response() {
        let response = parse(serde_json::json!({
            "outcome": "authorized",
            "transactionReference": "ref-1",
            "paymentInstrument": {
                "type": "card/plain+masked",
                "cardBin": "444433",
                "lastFour": "1111"
            },
            "_links": {
                "self": { "href": "https://try.access.worldpay.com/payments/authorizations/eyJrIjoi" }
            },
            "_actions": {
                "cancelPayment": {
                    "href": "https://try.access.worldpay.com/payments/authorizations/cancellations/eyJrIjoi",
                    "method": "POST"
                }
            }
        }));

        assert_eq!(response.outcome, PaymentOutcome::Authorized);
        assert_eq!(response.transaction_reference.as_deref(), Some("ref-1"));
        assert!(matches!(
            response.other_fields,
            Some(WorldpayPaymentResponseFields::AuthorizedResponse(_))
        ));
    }

    #[test]
    fn device_data_required_is_parsed_as_ddc_response() {
        let response = parse(serde_json::json!({
            "outcome": "3dsDeviceDataRequired",
            "deviceDataCollection": {
                "jwt": "jwt-token",
                "url": "https://ddc.example.com/collect",
                "bin": "444433"
            },
            "_actions": {
                "supply3dsDeviceData": {
                    "href": "https://try.access.worldpay.com/verifications/customers/3ds/deviceDataInitialization/eyJrIjoi/devicedata",
                    "method": "POST"
                }
            }
        }));

        assert_eq!(response.outcome, PaymentOutcome::ThreeDsDeviceDataRequired);
        assert!(matches!(
            response.other_fields,
            Some(WorldpayPaymentResponseFields::DDCResponse(_))
        ));
    }

    #[test]
    fn challenged_is_parsed_as_three_ds_challenged_response() {
        let response = parse(serde_json::json!({
            "outcome": "3dsChallenged",
            "authentication": { "version": "2.2.0" },
            "challenge": {
                "reference": "ch-1",
                "url": "https://acs.example.com/challenge",
                "jwt": "jwt-token",
                "payload": "payload"
            },
            "_actions": {
                "complete3dsChallenge": {
                    "href": "https://try.access.worldpay.com/payments/authorizations/eyJrIjoi/3dsChallenges",
                    "method": "POST"
                }
            }
        }));

        assert!(matches!(
            response.other_fields,
            Some(WorldpayPaymentResponseFields::ThreeDsChallenged(_))
        ));
    }

    #[test]
    fn fraud_high_risk_is_parsed_as_fraud_high_risk_response() {
        let response = parse(serde_json::json!({
            "outcome": "fraudHighRisk",
            "score": 97.5,
            "reason": ["Unusual transaction for merchant"]
        }));

        assert!(matches!(
            response.other_fields,
            Some(WorldpayPaymentResponseFields::FraudHighRisk(_))
        ));
    }

    #[test]
    fn outcome_without_extra_fields_has_no_other_fields() {
        let response = parse(serde_json::json!({
            "outcome": "sentForCancellation",
            "_links": {
                "self": { "href": "https://try.access.worldpay.com/payments/events/eyJrIjoi" }
            }
        }));

        assert_eq!(response.outcome, PaymentOutcome::SentForCancellation);
        assert_eq!(response.other_fields, None);
    }

    #[test]
    fn refused_with_mistyped_field_has_no_other_fields() {
        let response = parse(serde_json::json!({
            "outcome": "refused",
            "refusalCode": 5,
            "refusalDescription": "Refused"
        }));

        assert_eq!(response.outcome, PaymentOutcome::Refused);
        assert_eq!(response.other_fields, None);
    }

    #[test]
    fn deserialization_error_detail_keeps_missing_field_name() {
        let error = serde_json::from_value::<FraudHighRiskResponse>(serde_json::json!({
            "score": 97.5
        }))
        .unwrap_err();

        assert_eq!(
            deserialization_error_detail(&error),
            "missing field `reason`"
        );
    }

    #[test]
    fn deserialization_error_detail_omits_response_values() {
        let error = serde_json::from_value::<FraudHighRiskResponse>(serde_json::json!({
            "score": "secret-value-123",
            "reason": []
        }))
        .unwrap_err();

        let detail = deserialization_error_detail(&error);
        assert!(
            !detail.contains("secret-value-123"),
            "detail leaked a value: {detail}"
        );
    }

    #[test]
    fn missing_outcome_is_rejected() {
        let result = serde_json::from_value::<WorldpayPaymentsResponse>(serde_json::json!({
            "refusalCode": "5",
            "refusalDescription": "Refused"
        }));

        assert!(result.is_err());
    }
}
