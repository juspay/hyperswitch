use api_models::payments::AddressDetails;
use common_enums::CountryAlpha2;
use masking::Secret;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    UNKNOWN,
}

/// Represents the type of a payment instrument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixPaymentInstrumentType {
    #[serde(rename = "PAYMENT_CARD")]
    PaymentCard,

    #[serde(rename = "BANK_ACCOUNT")]
    BankAccount,
}

/// Represents the type of a payment card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixCardType {
    DEBIT,
    CREDIT,
    PREPAID,
}

/// Represents the brand of a payment card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixCardBrand {
    Visa,
    Mastercard,
    AmericanExpress,
    Discover,
    JCB,
    DinersClub,
}

/// 3D Secure authentication details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixThreeDSecure {
    pub authenticated: Option<bool>,
    pub liability_shift: Option<String>,
    pub version: Option<String>,
    pub eci: Option<String>,
    pub cavv: Option<String>,
    pub xid: Option<String>,
}

/// Key-value pair tags.
pub type FinixTags = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinixAddress {
    pub line1: Secret<String>,
    pub line2: Option<Secret<String>>,
    pub city: String,
    pub region: Secret<String>,
    pub postal_code: Secret<String>,
    pub country: CountryAlpha2,
}

impl From<&AddressDetails> for FinixAddress {
    fn from(address: &AddressDetails) -> Self {
        Self {
            line1: address.line1.clone().unwrap_or_default(),
            line2: address.line2.clone(),
            city: address.city.clone().unwrap_or_default(),
            region: address.state.clone().unwrap_or_default(),
            postal_code: address.zip.clone().unwrap_or_default(),
            country: address.country.unwrap_or(CountryAlpha2::US), //todo see is this is required
        }
    }
}

/// The type of the business.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixBusinessType {
    #[serde(rename = "SOLE_PROPRIETORSHIP")]
    SoleProprietorship,
    PARTNERSHIP,
    LLC,
    CORPORATION,
}

/// The type of the business.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixIdentityType {
    PERSONAL,
}
