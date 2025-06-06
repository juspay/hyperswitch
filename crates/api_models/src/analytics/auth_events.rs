use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use common_enums::{
    AuthenticationConnectors, AuthenticationStatus, Currency, DecoupledAuthenticationType,
    TransactionStatus,
};

use super::{NameDescription, TimeRange};

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct AuthEventFilters {
    #[serde(default)]
    pub authentication_status: Vec<AuthenticationStatus>,
    #[serde(default)]
    pub trans_status: Vec<TransactionStatus>,
    #[serde(default)]
    pub authentication_type: Vec<DecoupledAuthenticationType>,
    #[serde(default)]
    pub error_message: Vec<String>,
    #[serde(default)]
    pub authentication_connector: Vec<AuthenticationConnectors>,
    #[serde(default)]
    pub message_version: Vec<String>,
    #[serde(default)]
    pub platform: Vec<String>,
    #[serde(default)]
    pub acs_reference_number: Vec<String>,
    #[serde(default)]
    pub mcc: Vec<String>,
    #[serde(default)]
    pub currency: Vec<Currency>,
    #[serde(default)]
    pub merchant_country: Vec<String>,
    #[serde(default)]
    pub billing_country: Vec<String>,
    #[serde(default)]
    pub shipping_country: Vec<String>,
    #[serde(default)]
    pub issuer_country: Vec<String>,
    #[serde(default)]
    pub earliest_supported_version: Vec<String>,
    #[serde(default)]
    pub latest_supported_version: Vec<String>,
    #[serde(default)]
    pub whitelist_decision: Vec<bool>,
    #[serde(default)]
    pub device_manufacturer: Vec<String>,
    #[serde(default)]
    pub device_type: Vec<String>,
    #[serde(default)]
    pub device_brand: Vec<String>,
    #[serde(default)]
    pub device_os: Vec<String>,
    #[serde(default)]
    pub device_display: Vec<String>,
    #[serde(default)]
    pub browser_name: Vec<String>,
    #[serde(default)]
    pub browser_version: Vec<String>,
    #[serde(default)]
    pub issuer_id: Vec<String>,
    #[serde(default)]
    pub scheme_name: Vec<String>,
    #[serde(default)]
    pub exemption_requested: Vec<bool>,
    #[serde(default)]
    pub exemption_accepted: Vec<bool>,
}

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    strum::AsRefStr,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    strum::Display,
    strum::EnumIter,
    Clone,
    Copy,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AuthEventDimensions {
    AuthenticationStatus,
    #[strum(serialize = "trans_status")]
    #[serde(rename = "trans_status")]
    TransactionStatus,
    AuthenticationType,
    ErrorMessage,
    AuthenticationConnector,
    MessageVersion,
    AcsReferenceNumber,
    Platform,
    Mcc,
    Currency,
    MerchantCountry,
    BillingCountry,
    ShippingCountry,
    IssuerCountry,
    EarliestSupportedVersion,
    LatestSupportedVersion,
    WhitelistDecision,
    DeviceManufacturer,
    DeviceType,
    DeviceBrand,
    DeviceOs,
    DeviceDisplay,
    BrowserName,
    BrowserVersion,
    IssuerId,
    SchemeName,
    ExemptionRequested,
    ExemptionAccepted,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumIter,
    strum::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AuthEventMetrics {
    AuthenticationCount,
    AuthenticationAttemptCount,
    AuthenticationSuccessCount,
    ChallengeFlowCount,
    FrictionlessFlowCount,
    FrictionlessSuccessCount,
    ChallengeAttemptCount,
    ChallengeSuccessCount,
    AuthenticationErrorMessage,
    AuthenticationFunnel,
    AuthenticationExemptionApprovedCount,
    AuthenticationExemptionRequestedCount,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumIter,
    strum::AsRefStr,
)]
pub enum AuthEventFlows {
    IncomingWebhookReceive,
    PaymentsExternalAuthentication,
}

pub mod metric_behaviour {
    pub struct AuthenticationCount;
    pub struct AuthenticationAttemptCount;
    pub struct AuthenticationSuccessCount;
    pub struct ChallengeFlowCount;
    pub struct FrictionlessFlowCount;
    pub struct FrictionlessSuccessCount;
    pub struct ChallengeAttemptCount;
    pub struct ChallengeSuccessCount;
    pub struct AuthenticationErrorMessage;
    pub struct AuthenticationFunnel;
}

impl From<AuthEventMetrics> for NameDescription {
    fn from(value: AuthEventMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

impl From<AuthEventDimensions> for NameDescription {
    fn from(value: AuthEventDimensions) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct AuthEventMetricsBucketIdentifier {
    pub authentication_status: Option<AuthenticationStatus>,
    pub trans_status: Option<TransactionStatus>,
    pub authentication_type: Option<DecoupledAuthenticationType>,
    pub error_message: Option<String>,
    pub authentication_connector: Option<AuthenticationConnectors>,
    pub message_version: Option<String>,
    pub acs_reference_number: Option<String>,
    pub mcc: Option<String>,
    pub currency: Option<Currency>,
    pub merchant_country: Option<String>,
    pub billing_country: Option<String>,
    pub shipping_country: Option<String>,
    pub issuer_country: Option<String>,
    pub earliest_supported_version: Option<String>,
    pub latest_supported_version: Option<String>,
    pub whitelist_decision: Option<bool>,
    pub device_manufacturer: Option<String>,
    pub device_type: Option<String>,
    pub device_brand: Option<String>,
    pub device_os: Option<String>,
    pub device_display: Option<String>,
    pub browser_name: Option<String>,
    pub browser_version: Option<String>,
    pub issuer_id: Option<String>,
    pub scheme_name: Option<String>,
    pub exemption_requested: Option<bool>,
    pub exemption_accepted: Option<bool>,
    #[serde(rename = "time_range")]
    pub time_bucket: TimeRange,
    #[serde(rename = "time_bucket")]
    #[serde(with = "common_utils::custom_serde::iso8601custom")]
    pub start_time: time::PrimitiveDateTime,
}

impl AuthEventMetricsBucketIdentifier {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        authentication_status: Option<AuthenticationStatus>,
        trans_status: Option<TransactionStatus>,
        authentication_type: Option<DecoupledAuthenticationType>,
        error_message: Option<String>,
        authentication_connector: Option<AuthenticationConnectors>,
        message_version: Option<String>,
        acs_reference_number: Option<String>,
        mcc: Option<String>,
        currency: Option<Currency>,
        merchant_country: Option<String>,
        billing_country: Option<String>,
        shipping_country: Option<String>,
        issuer_country: Option<String>,
        earliest_supported_version: Option<String>,
        latest_supported_version: Option<String>,
        whitelist_decision: Option<bool>,
        device_manufacturer: Option<String>,
        device_type: Option<String>,
        device_brand: Option<String>,
        device_os: Option<String>,
        device_display: Option<String>,
        browser_name: Option<String>,
        browser_version: Option<String>,
        issuer_id: Option<String>,
        scheme_name: Option<String>,
        exemption_requested: Option<bool>,
        exemption_accepted: Option<bool>,
        normalized_time_range: TimeRange,
    ) -> Self {
        Self {
            authentication_status,
            trans_status,
            authentication_type,
            error_message,
            authentication_connector,
            message_version,
            acs_reference_number,
            mcc,
            currency,
            merchant_country,
            billing_country,
            shipping_country,
            issuer_country,
            earliest_supported_version,
            latest_supported_version,
            whitelist_decision,
            device_manufacturer,
            device_type,
            device_brand,
            device_os,
            device_display,
            browser_name,
            browser_version,
            issuer_id,
            scheme_name,
            exemption_requested,
            exemption_accepted,
            time_bucket: normalized_time_range,
            start_time: normalized_time_range.start_time,
        }
    }
}

impl Hash for AuthEventMetricsBucketIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.authentication_status.hash(state);
        self.trans_status.hash(state);
        self.authentication_type.hash(state);
        self.authentication_connector.hash(state);
        self.message_version.hash(state);
        self.acs_reference_number.hash(state);
        self.error_message.hash(state);
        self.mcc.hash(state);
        self.currency.hash(state);
        self.merchant_country.hash(state);
        self.billing_country.hash(state);
        self.shipping_country.hash(state);
        self.issuer_country.hash(state);
        self.earliest_supported_version.hash(state);
        self.latest_supported_version.hash(state);
        self.whitelist_decision.hash(state);
        self.device_manufacturer.hash(state);
        self.device_type.hash(state);
        self.device_brand.hash(state);
        self.device_os.hash(state);
        self.device_display.hash(state);
        self.browser_name.hash(state);
        self.browser_version.hash(state);
        self.issuer_id.hash(state);
        self.scheme_name.hash(state);
        self.exemption_requested.hash(state);
        self.exemption_accepted.hash(state);
        self.time_bucket.hash(state);
    }
}

impl PartialEq for AuthEventMetricsBucketIdentifier {
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct AuthEventMetricsBucketValue {
    pub authentication_count: Option<u64>,
    pub authentication_attempt_count: Option<u64>,
    pub authentication_success_count: Option<u64>,
    pub challenge_flow_count: Option<u64>,
    pub challenge_attempt_count: Option<u64>,
    pub challenge_success_count: Option<u64>,
    pub frictionless_flow_count: Option<u64>,
    pub frictionless_success_count: Option<u64>,
    pub error_message_count: Option<u64>,
    pub authentication_funnel: Option<u64>,
    pub authentication_exemption_approved_count: Option<u64>,
    pub authentication_exemption_requested_count: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct MetricsBucketResponse {
    #[serde(flatten)]
    pub values: AuthEventMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: AuthEventMetricsBucketIdentifier,
}
