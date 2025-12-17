use std::collections::HashMap;

use common_enums::Currency;
use common_utils::{pii::Email, types::MinorUnit};
use masking::Secret;
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixPaymentsRequest {
    pub amount: MinorUnit,
    pub currency: Currency,
    pub source: Secret<String>,
    pub merchant: Secret<String>,
    pub tags: Option<FinixTags>,
    pub three_d_secure: Option<FinixThreeDSecure>,
    pub idempotency_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixCaptureRequest {
    pub capture_amount: MinorUnit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixCancelRequest {
    pub void_me: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixCaptureAuthorizationRequest {
    pub amount: Option<MinorUnit>,
    pub tags: Option<FinixTags>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixCreateIdentityRequest {
    pub entity: FinixIdentityEntity,
    pub tags: Option<FinixTags>,
    #[serde(rename = "type")]
    pub identity_type: FinixIdentityType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixIdentityEntity {
    pub phone: Option<Secret<String>>,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub email: Option<Email>,
    pub personal_address: Option<FinixAddress>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FinixApplePayPaymentToken {
    pub token: FinixApplePayToken,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinixApplePayHeader {
    pub public_key_hash: String,
    pub ephemeral_public_key: String,
    pub transaction_id: String,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinixApplePayEncryptedData {
    pub data: Secret<String>,
    pub signature: Secret<String>,
    pub header: FinixApplePayHeader,
    pub version: Secret<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinixApplePayPaymentMethod {
    pub display_name: Secret<String>,
    pub network: Secret<String>,
    #[serde(rename = "type")]
    pub method_type: Secret<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinixApplePayToken {
    pub payment_data: FinixApplePayEncryptedData,
    pub payment_method: FinixApplePayPaymentMethod,
    pub transaction_identifier: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixCreatePaymentInstrumentRequest {
    #[serde(rename = "type")]
    pub instrument_type: FinixPaymentInstrumentType,
    pub name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_code: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_month: Option<Secret<i8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_year: Option<Secret<i32>>,
    pub identity: String,
    pub tags: Option<FinixTags>,
    pub address: Option<FinixAddress>,
    pub card_brand: Option<String>,
    pub card_type: Option<FinixCardType>,
    pub additional_data: Option<HashMap<String, String>>,
    pub merchant_identity: Option<Secret<String>>,
    pub third_party_token: Option<Secret<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixCreateRefundRequest {
    pub refund_amount: MinorUnit,
}

impl FinixCreateRefundRequest {
    pub fn new(refund_amount: MinorUnit) -> Self {
        Self { refund_amount }
    }
}

// ---------- COMMON ENUMS

#[derive(Debug, Clone)]
pub enum FinixId {
    Auth(String),
    Transfer(String),
}

impl From<String> for FinixId {
    fn from(id: String) -> Self {
        if id.starts_with("AU") {
            Self::Auth(id)
        } else if id.starts_with("TR") {
            Self::Transfer(id)
        } else {
            // Default to Auth if the prefix doesn't match
            Self::Auth(id)
        }
    }
}

impl std::fmt::Display for FinixId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auth(id) => write!(f, "{}", id),
            Self::Transfer(id) => write!(f, "{}", id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixState {
    PENDING,
    SUCCEEDED,
    FAILED,
    CANCELED,
    #[serde(other)]
    UNKNOWN,
    // RETURNED
}
impl FinixState {
    pub fn is_failure(&self) -> bool {
        match self {
            Self::PENDING | Self::SUCCEEDED => false,
            Self::FAILED | Self::CANCELED | Self::UNKNOWN => true,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixPaymentType {
    DEBIT,
    CREDIT,
    REVERSAL,
    FEE,
    ADJUSTMENT,
    DISPUTE,
    RESERVE,
    SETTLEMENT,
    #[serde(other)]
    UNKNOWN,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixPaymentInstrumentType {
    #[serde(rename = "PAYMENT_CARD")]
    PaymentCard,
    #[serde(rename = "GOOGLE_PAY")]
    GOOGLEPAY,

    #[serde(rename = "BANK_ACCOUNT")]
    BankAccount,

    #[serde(rename = "APPLE_PAY")]
    ApplePay,

    #[serde(other)]
    Unknown,
}

/// Represents the type of a payment card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixCardType {
    DEBIT,
    CREDIT,
    PREPAID,
    #[serde(other)]
    UNKNOWN,
}

/// 3D Secure authentication details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixThreeDSecure {
    pub authenticated: Option<bool>,
    pub liability_shift: Option<Secret<String>>,
    pub version: Option<String>,
    pub eci: Option<Secret<String>>,
    pub cavv: Option<Secret<String>>,
    pub xid: Option<Secret<String>>,
}

/// Key-value pair tags.
pub type FinixTags = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixAddress {
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub city: Option<String>,
    pub region: Option<Secret<String>>,
    pub postal_code: Option<Secret<String>>,
    pub country: Option<CountryAlpha3>,
}

/// The type of the business.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixIdentityType {
    PERSONAL,
}

pub enum FinixFlow {
    Auth,
    Transfer,
    Capture,
}

impl FinixFlow {
    pub fn get_flow_for_auth(capture_method: CaptureMethod) -> Self {
        match capture_method {
            CaptureMethod::SequentialAutomatic | CaptureMethod::Automatic => Self::Transfer,
            CaptureMethod::Manual | CaptureMethod::ManualMultiple | CaptureMethod::Scheduled => {
                Self::Auth
            }
        }
    }
}
pub struct FinixAuthType {
    pub finix_user_name: Secret<String>,
    pub finix_password: Secret<String>,
    pub merchant_id: Secret<String>,
    pub merchant_identity_id: Secret<String>,
}
