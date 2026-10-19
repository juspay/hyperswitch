use common_enums::CountryAlpha2;
use common_utils::{
    new_type::{MaskedEmail, MaskedPhoneNumber},
    pii::Email,
    transformers::ForeignFrom,
};
use hyperswitch_masking::Secret;
use time::Date;
use utoipa::ToSchema;

use crate::payments::PhoneDetails;

/// The address of the party providing the funds in an account funded transaction.
#[derive(Eq, PartialEq, Clone, Debug, Default, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SenderAddress {
    /// The first line of the sender's address.
    #[schema(value_type = Option<String>, max_length = 200, example = "123, King Street")]
    pub line1: Option<Secret<String>>,

    /// The second line of the sender's address.
    #[schema(value_type = Option<String>, max_length = 200, example = "Powelson Avenue")]
    pub line2: Option<Secret<String>>,

    /// The sender's city.
    #[schema(max_length = 50, example = "London")]
    pub city: Option<String>,

    /// The sender's state or province.
    #[schema(value_type = Option<String>, example = "CA")]
    pub state: Option<Secret<String>>,

    /// The sender's zip or postal code.
    #[schema(value_type = Option<String>, max_length = 50, example = "EC1A 1BB")]
    pub zip: Option<Secret<String>>,

    /// The two-letter ISO country code of the sender's address.
    #[schema(value_type = Option<CountryAlpha2>, example = "GB")]
    pub country: Option<CountryAlpha2>,
}

impl SenderAddress {
    /// Returns true when none of the address fields are provided.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// An individual providing the funds in an account funded transaction.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct IndividualSenderDetails {
    /// The sender's first name.
    #[schema(value_type = String, max_length = 255, example = "John")]
    pub first_name: Secret<String>,

    /// The sender's last name.
    #[schema(value_type = String, max_length = 255, example = "Doe")]
    pub last_name: Secret<String>,

    /// The sender's date of birth, in the format yyyy-mm-dd.
    #[schema(value_type = Option<Date>, example = "1990-01-31")]
    pub date_of_birth: Option<Secret<Date>>,

    /// The sender's address. When no address field is provided, the billing address of the
    /// payment method is used instead.
    #[schema(value_type = Option<SenderAddress>)]
    pub address: Option<SenderAddress>,

    /// The sender's phone number.
    #[schema(value_type = Option<PhoneDetails>)]
    pub phone: Option<PhoneDetails>,

    /// The sender's email address.
    #[schema(value_type = Option<String>, example = "john.doe@example.com")]
    pub email: Option<Email>,
}

/// A company providing the funds in an account funded transaction.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CorporateSenderDetails {
    /// The sender's legal company name.
    #[schema(value_type = String, max_length = 255, example = "Acme Ltd")]
    pub company_name: Secret<String>,

    /// The sender's address. When no address field is provided, the billing address of the
    /// payment method is used instead.
    #[schema(value_type = Option<SenderAddress>)]
    pub address: Option<SenderAddress>,

    /// The sender's phone number.
    #[schema(value_type = Option<PhoneDetails>)]
    pub phone: Option<PhoneDetails>,

    /// The sender's email address.
    #[schema(value_type = Option<String>, example = "treasury@acme.com")]
    pub email: Option<Email>,
}

/// Details of the party providing the funds in an account funded transaction.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SenderDetails {
    /// The funds are provided by an individual.
    Individual(IndividualSenderDetails),
    /// The funds are provided by a company.
    Corporate(CorporateSenderDetails),
}

impl SenderDetails {
    /// The names identifying the sender, keyed by their field path, for validation.
    pub fn get_name_fields(&self) -> Vec<(&'static str, &Secret<String>)> {
        match self {
            Self::Individual(individual) => vec![
                ("sender_details.first_name", &individual.first_name),
                ("sender_details.last_name", &individual.last_name),
            ],
            Self::Corporate(corporate) => {
                vec![("sender_details.company_name", &corporate.company_name)]
            }
        }
    }

    /// The sender's address, if provided.
    pub fn get_address(&self) -> Option<&SenderAddress> {
        match self {
            Self::Individual(individual) => individual.address.as_ref(),
            Self::Corporate(corporate) => corporate.address.as_ref(),
        }
    }
}

/// A partially masked view of `PhoneDetails`, safe to return in API responses.
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
pub struct MaskedIndividualSenderDetails {
    /// The sender's first name.
    #[schema(value_type = String, example = "John")]
    pub first_name: Secret<String>,

    /// The sender's last name.
    #[schema(value_type = String, example = "Doe")]
    pub last_name: Secret<String>,

    /// The sender's address.
    #[schema(value_type = Option<SenderAddress>)]
    pub address: Option<SenderAddress>,

    /// The sender's partially masked phone number.
    #[schema(value_type = Option<MaskedSenderPhoneDetails>)]
    pub phone: Option<MaskedSenderPhoneDetails>,

    /// The sender's masked email address.
    #[schema(value_type = Option<String>, example = "jo******@example.com")]
    pub email: Option<MaskedEmail>,
}

/// A partially masked view of `CorporateSenderDetails`, safe to return in API responses.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MaskedCorporateSenderDetails {
    /// The sender's legal company name.
    #[schema(value_type = String, example = "Acme Ltd")]
    pub company_name: Secret<String>,

    /// The sender's address.
    #[schema(value_type = Option<SenderAddress>)]
    pub address: Option<SenderAddress>,

    /// The sender's partially masked phone number.
    #[schema(value_type = Option<MaskedSenderPhoneDetails>)]
    pub phone: Option<MaskedSenderPhoneDetails>,

    /// The sender's masked email address.
    #[schema(value_type = Option<String>, example = "tr******@acme.com")]
    pub email: Option<MaskedEmail>,
}

/// A partially masked view of `SenderDetails`, safe to return in API responses. The sender's date
/// of birth is never returned.
#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MaskedSenderDetails {
    /// The funds are provided by an individual.
    Individual(MaskedIndividualSenderDetails),
    /// The funds are provided by a company.
    Corporate(MaskedCorporateSenderDetails),
}

impl From<PhoneDetails> for MaskedSenderPhoneDetails {
    fn from(phone: PhoneDetails) -> Self {
        Self {
            number: phone.number.map(MaskedPhoneNumber::from),
            country_code: phone.country_code,
        }
    }
}

impl From<SenderDetails> for MaskedSenderDetails {
    fn from(sender_details: SenderDetails) -> Self {
        match sender_details {
            SenderDetails::Individual(individual) => {
                Self::Individual(MaskedIndividualSenderDetails {
                    first_name: individual.first_name,
                    last_name: individual.last_name,
                    address: individual.address,
                    phone: individual.phone.map(MaskedSenderPhoneDetails::from),
                    email: individual.email.map(MaskedEmail::foreign_from),
                })
            }
            SenderDetails::Corporate(corporate) => Self::Corporate(MaskedCorporateSenderDetails {
                company_name: corporate.company_name,
                address: corporate.address,
                phone: corporate.phone.map(MaskedSenderPhoneDetails::from),
                email: corporate.email.map(MaskedEmail::foreign_from),
            }),
        }
    }
}
