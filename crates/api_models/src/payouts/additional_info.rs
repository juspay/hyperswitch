use common_utils::new_type::{
    MaskedBankAccount, MaskedBic, MaskedEmail, MaskedIban, MaskedPhoneNumber, MaskedRoutingNumber,
    MaskedSortCode,
};
use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::enums as api_enums;

#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
pub enum AdditionalPayoutMethodData {
    Card(Box<CardAdditionalData>),
    Bank(Box<BankAdditionalData>),
    Wallet(Box<WalletAdditionalData>),
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CardAdditionalData {
    /// Issuer of the card
    pub card_issuer: Option<String>,

    /// Card network of the card
    #[schema(value_type = Option<CardNetwork>)]
    pub card_network: Option<api_enums::CardNetwork>,

    /// Card type, can be either `credit` or `debit`
    pub card_type: Option<String>,

    /// Card issuing country
    pub card_issuing_country: Option<String>,

    /// Code for Card issuing bank
    pub bank_code: Option<String>,

    /// Last 4 digits of the card number
    pub last4: Option<String>,

    /// The ISIN of the card
    pub card_isin: Option<String>,

    /// Extended bin of card, contains the first 8 digits of card number
    pub card_extended_bin: Option<String>,

    /// Card expiry month
    #[schema(value_type = String, example = "01")]
    pub card_exp_month: Option<Secret<String>>,

    /// Card expiry year
    #[schema(value_type = String, example = "2026")]
    pub card_exp_year: Option<Secret<String>>,

    /// Card holder name
    #[schema(value_type = String, example = "John Doe")]
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(untagged)]
pub enum BankAdditionalData {
    Ach(Box<AchBankTransferAdditionalData>),
    Bacs(Box<BacsBankTransferAdditionalData>),
    Sepa(Box<SepaBankTransferAdditionalData>),
    Pix(Box<PixBankTransferAdditionalData>),
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AchBankTransferAdditionalData {
    /// Partially masked account number for ach bank debit payment
    #[schema(value_type = String, example = "0001****3456")]
    pub bank_account_number: MaskedBankAccount,

    /// Partially masked routing number for ach bank debit payment
    #[schema(value_type = String, example = "110***000")]
    pub bank_routing_number: MaskedRoutingNumber,

    /// Name of the bank
    #[schema(value_type = Option<BankNames>, example = "Deutsche Bank")]
    pub bank_name: Option<String>,

    /// Bank country code
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub bank_country_code: Option<api_enums::CountryAlpha2>,

    /// Bank city
    #[schema(value_type = Option<String>, example = "California")]
    pub bank_city: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BacsBankTransferAdditionalData {
    /// Partially masked sort code for Bacs payment method
    #[schema(value_type = String, example = "108800")]
    pub bank_sort_code: MaskedSortCode,

    /// Bank account's owner name
    #[schema(value_type = String, example = "0001****3456")]
    pub bank_account_number: MaskedBankAccount,

    /// Bank name
    #[schema(value_type = Option<String>, example = "Deutsche Bank")]
    pub bank_name: Option<String>,

    /// Bank country code
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub bank_country_code: Option<api_enums::CountryAlpha2>,

    /// Bank city
    #[schema(value_type = Option<String>, example = "California")]
    pub bank_city: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SepaBankTransferAdditionalData {
    /// Partially masked international bank account number (iban) for SEPA
    #[schema(value_type = String, example = "DE8937******013000")]
    pub iban: MaskedIban,

    /// Bank name
    #[schema(value_type = Option<String>, example = "Deutsche Bank")]
    pub bank_name: Option<String>,

    /// Bank country code
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub bank_country_code: Option<api_enums::CountryAlpha2>,

    /// Bank city
    #[schema(value_type = Option<String>, example = "California")]
    pub bank_city: Option<String>,

    /// [8 / 11 digits] Bank Identifier Code (bic) / Swift Code - used in many countries for identifying a bank and it's branches
    #[schema(value_type = Option<String>, example = "HSBCGB2LXXX")]
    pub bic: Option<MaskedBic>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PixBankTransferAdditionalData {
    /// Partially masked unique key for pix transfer
    #[schema(value_type = String, example = "a1f4102e ****** 6fa48899c1d1")]
    pub pix_key: MaskedBankAccount,

    /// Partially masked CPF - CPF is a Brazilian tax identification number
    #[schema(value_type = Option<String>, example = "**** 124689")]
    pub tax_id: Option<MaskedBankAccount>,

    /// Bank account number is an unique identifier assigned by a bank to a customer.
    #[schema(value_type = String, example = "**** 23456")]
    pub bank_account_number: MaskedBankAccount,

    /// Bank name
    #[schema(value_type = Option<String>, example = "Deutsche Bank")]
    pub bank_name: Option<String>,

    /// Bank branch
    #[schema(value_type = Option<String>, example = "3707")]
    pub bank_branch: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WalletAdditionalData {
    Paypal(Box<PaypalAdditionalData>),
    Venmo(Box<VenmoAdditionalData>),
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PaypalAdditionalData {
    /// Email linked with paypal account
    #[schema(value_type = Option<String>, example = "john.doe@example.com")]
    pub email: Option<MaskedEmail>,

    /// mobile number linked to paypal account
    #[schema(value_type = Option<String>, example = "******* 3349")]
    pub telephone_number: Option<MaskedPhoneNumber>,

    /// id of the paypal account
    #[schema(value_type = Option<String>, example = "G83K ***** HCQ2")]
    pub paypal_id: Option<MaskedBankAccount>,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct VenmoAdditionalData {
    /// mobile number linked to venmo account
    #[schema(value_type = Option<String>, example = "******* 3349")]
    pub telephone_number: Option<MaskedPhoneNumber>,
}
