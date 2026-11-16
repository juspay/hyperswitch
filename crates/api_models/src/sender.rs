//! Sender identity — the party providing the funds, independent of the destination.
//!
//! These types are shared by payments (account funded transactions) and payouts. In payments
//! the sender is persisted encrypted on the payment intent and echoed in responses only
//! through the masked [`SenderDetailsResponse`] view; in payouts the sender is persisted
//! encrypted on the payout and normalized into common payout vendor details at the domain
//! boundary.

use common_types::payments::PhoneDetails;
use common_utils::{
    new_type::{MaskedEmail, MaskedPhoneNumber},
    pii::Email,
    transformers::ForeignFrom,
};
use hyperswitch_masking::Secret;
use time::Date;
use utoipa::ToSchema;

use crate::payments::AddressDetails;

/// Origin of the funds, supplied by the merchant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceOfFunds {
    /// Credit funding.
    Credit,
    /// Debit funding.
    Debit,
    /// Prepaid funding.
    Prepaid,
    /// Deposit account funding.
    DepositAccount,
    /// Mobile money funding.
    MobileMoneyAccount,
    /// Cash funding.
    Cash,
}

/// An individual providing the funds.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct IndividualSenderDetails {
    /// The sender's first name. When neither the first nor the last name is provided, the name
    /// from the billing address of the payment method is used instead.
    #[schema(value_type = Option<String>, max_length = 255, example = "John")]
    pub first_name: Option<Secret<String>>,

    /// The sender's last name. When neither the first nor the last name is provided, the name
    /// from the billing address of the payment method is used instead.
    #[schema(value_type = Option<String>, max_length = 255, example = "Doe")]
    pub last_name: Option<Secret<String>>,

    /// The sender's date of birth, in the format yyyy-mm-dd.
    #[schema(value_type = Option<Date>, example = "1990-01-31")]
    pub date_of_birth: Option<Secret<Date>>,

    /// The sender's address. When no address field is provided, the billing address of the
    /// payment method is used instead, unless the sender is named and their name differs from
    /// the billing name, in which case no address is sent.
    #[schema(value_type = Option<AddressDetails>)]
    pub address: Option<AddressDetails>,

    /// The sender's email address.
    #[schema(value_type = Option<String>, example = "john.doe@example.com")]
    pub email: Option<Email>,

    /// The sender's phone number.
    #[schema(value_type = Option<PhoneDetails>)]
    pub phone: Option<PhoneDetails>,

    /// The merchant's stable identifier for the sender
    #[schema(value_type = Option<String>)]
    pub reference_id: Option<Secret<String>>,

    /// Origin of the funds, supplied by the merchant.
    pub source_of_funds: Option<SourceOfFunds>,
}

/// A company providing the funds.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CorporateSenderDetails {
    /// The sender's legal company name.
    #[schema(value_type = String, max_length = 255, example = "Acme Ltd")]
    pub company_name: Secret<String>,

    /// The sender's address. When no address field is provided, the billing address of the
    /// payment method is used instead.
    #[schema(value_type = Option<AddressDetails>)]
    pub address: Option<AddressDetails>,

    /// The sender's email address.
    #[schema(value_type = Option<String>, example = "treasury@acme.com")]
    pub email: Option<Email>,

    /// The sender's phone number.
    #[schema(value_type = Option<PhoneDetails>)]
    pub phone: Option<PhoneDetails>,

    /// The merchant's stable identifier for the sender.
    #[schema(value_type = Option<String>)]
    pub reference_id: Option<Secret<String>>,

    /// Origin of the funds, supplied by the merchant.
    pub source_of_funds: Option<SourceOfFunds>,
}

/// Details of the party providing the funds.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SenderDetails {
    /// The funds are provided by an individual.
    Individual(IndividualSenderDetails),
    /// The funds are provided by a company.
    Corporate(CorporateSenderDetails),
}

impl SenderDetails {
    /// The individual sender's first name, if provided.
    pub fn get_individual_first_name(&self) -> Option<&Secret<String>> {
        match self {
            Self::Individual(individual) => individual.first_name.as_ref(),
            Self::Corporate(_) => None,
        }
    }

    /// The individual sender's last name, if provided.
    pub fn get_individual_last_name(&self) -> Option<&Secret<String>> {
        match self {
            Self::Individual(individual) => individual.last_name.as_ref(),
            Self::Corporate(_) => None,
        }
    }

    /// The corporate sender's company name, if applicable.
    pub fn get_corporate_company_name(&self) -> Option<&Secret<String>> {
        match self {
            Self::Individual(_) => None,
            Self::Corporate(corporate) => Some(&corporate.company_name),
        }
    }

    /// The sender's address, if provided.
    pub fn get_address(&self) -> Option<&AddressDetails> {
        match self {
            Self::Individual(individual) => individual.address.as_ref(),
            Self::Corporate(corporate) => corporate.address.as_ref(),
        }
    }

    /// The sender's email address, if provided.
    pub fn get_email(&self) -> Option<&Email> {
        match self {
            Self::Individual(individual) => individual.email.as_ref(),
            Self::Corporate(corporate) => corporate.email.as_ref(),
        }
    }

    /// The merchant-supplied sender reference, independent of the sender kind.
    pub fn reference_id(&self) -> Option<&Secret<String>> {
        match self {
            Self::Individual(sender) => sender.reference_id.as_ref(),
            Self::Corporate(sender) => sender.reference_id.as_ref(),
        }
    }
}

/// A partially masked view of [`PhoneDetails`], safe to return in API responses.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MaskedSenderPhoneDetails {
    /// The sender's partially masked phone number.
    #[schema(value_type = Option<String>, example = "********89")]
    pub number: Option<MaskedPhoneNumber>,

    /// The country code of the sender's phone number.
    #[schema(example = "+44")]
    pub country_code: Option<String>,
}

/// A partially masked view of `IndividualSenderDetails`, safe to return in API responses.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct IndividualSenderDetailsResponse {
    /// The sender's first name.
    #[schema(value_type = Option<String>, example = "John")]
    pub first_name: Option<Secret<String>>,

    /// The sender's last name.
    #[schema(value_type = Option<String>, example = "Doe")]
    pub last_name: Option<Secret<String>>,

    /// The sender's address.
    #[schema(value_type = Option<AddressDetails>)]
    pub address: Option<AddressDetails>,

    /// The sender's partially masked phone number.
    #[schema(value_type = Option<MaskedSenderPhoneDetails>)]
    pub phone: Option<MaskedSenderPhoneDetails>,

    /// The sender's masked email address.
    #[schema(value_type = Option<String>, example = "jo******@example.com")]
    pub email: Option<MaskedEmail>,
}

/// A partially masked view of `CorporateSenderDetails`, safe to return in API responses.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CorporateSenderDetailsResponse {
    /// The sender's legal company name.
    #[schema(value_type = String, example = "Acme Ltd")]
    pub company_name: Secret<String>,

    /// The sender's address.
    #[schema(value_type = Option<AddressDetails>)]
    pub address: Option<AddressDetails>,

    /// The sender's partially masked phone number.
    #[schema(value_type = Option<MaskedSenderPhoneDetails>)]
    pub phone: Option<MaskedSenderPhoneDetails>,

    /// The sender's masked email address.
    #[schema(value_type = Option<String>, example = "tr******@acme.com")]
    pub email: Option<MaskedEmail>,
}

/// A partially masked view of `SenderDetails`, safe to return in API responses. The sender's
/// date of birth is never returned.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SenderDetailsResponse {
    /// The funds are provided by an individual.
    Individual(IndividualSenderDetailsResponse),
    /// The funds are provided by a company.
    Corporate(CorporateSenderDetailsResponse),
}

impl From<PhoneDetails> for MaskedSenderPhoneDetails {
    fn from(phone: PhoneDetails) -> Self {
        Self {
            number: phone.number.map(MaskedPhoneNumber::from),
            country_code: phone.country_code,
        }
    }
}

impl From<SenderDetails> for SenderDetailsResponse {
    fn from(sender_details: SenderDetails) -> Self {
        match sender_details {
            SenderDetails::Individual(individual) => {
                Self::Individual(IndividualSenderDetailsResponse {
                    first_name: individual.first_name,
                    last_name: individual.last_name,
                    address: individual.address,
                    phone: individual.phone.map(MaskedSenderPhoneDetails::from),
                    email: individual.email.map(MaskedEmail::foreign_from),
                })
            }
            SenderDetails::Corporate(corporate) => {
                Self::Corporate(CorporateSenderDetailsResponse {
                    company_name: corporate.company_name,
                    address: corporate.address,
                    phone: corporate.phone.map(MaskedSenderPhoneDetails::from),
                    email: corporate.email.map(MaskedEmail::foreign_from),
                })
            }
        }
    }
}
