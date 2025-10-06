use common_enums::{CountryAlpha2, CountryAlpha3};
use hyperswitch_domain_models::address::Address;
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
    #[serde(other)]
    UNKNOWN,
    // RETURNED
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

    #[serde(rename = "BANK_ACCOUNT")]
    BankAccount,
}

/// Represents the type of a payment card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinixCardType {
    DEBIT,
    CREDIT,
    PREPAID,
    UNKNOWN,
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
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub city: Option<String>,
    pub region: Option<Secret<String>>,
    pub postal_code: Option<Secret<String>>,
    pub country: Option<CountryAlpha3>,
}

impl From<&Address> for FinixAddress {
    fn from(address: &Address) -> Self {
        let billing = address.address.as_ref();

        match billing {
            Some(address) => Self {
                line1: address.line1.clone(),
                line2: address.line2.clone(),
                city: address.city.clone(),
                region: address.state.clone(),
                postal_code: address.zip.clone(),
                country: address
                    .country
                    .clone()
                    .map(CountryAlpha2::from_alpha2_to_alpha3),
            },
            None => Self {
                line1: None,
                line2: None,
                city: None,
                region: None,
                postal_code: None,
                country: None,
            },
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
