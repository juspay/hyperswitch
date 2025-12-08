//! This module has common utilities for payout method data in HyperSwitch

use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::new_type::{
    MaskedBankAccount, MaskedBic, MaskedEmail, MaskedIban, MaskedPhoneNumber, MaskedPspToken,
    MaskedRoutingNumber, MaskedSortCode,
};

/// Masked payout method details for storing in db
#[derive(
    Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub enum AdditionalPayoutMethodData {
    /// Additional data for card payout method
    Card(Box<CardAdditionalData>),
    /// Additional data for bank payout method
    Bank(Box<BankAdditionalData>),
    /// Additional data for wallet payout method
    Wallet(Box<WalletAdditionalData>),
    /// Additional data for Bank Redirect payout method
    BankRedirect(Box<BankRedirectAdditionalData>),
    /// Additional data for Passthrough payout method
    Passthrough(Box<PassthroughAddtionalData>),
}

crate::impl_to_sql_from_sql_json!(AdditionalPayoutMethodData);

/// Masked payout method details for card payout method
#[derive(
    Eq, PartialEq, Clone, Debug, Serialize, Deserialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct CardAdditionalData {
    /// Issuer of the card
    pub card_issuer: Option<String>,

    /// Card network of the card
    #[schema(value_type = Option<CardNetwork>)]
    pub card_network: Option<common_enums::CardNetwork>,

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

/// Masked payout method details for bank payout method
#[derive(
    Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(untagged)]
pub enum BankAdditionalData {
    /// Additional data for ach bank transfer payout method
    Ach(Box<AchBankTransferAdditionalData>),
    /// Additional data for bacs bank transfer payout method
    Bacs(Box<BacsBankTransferAdditionalData>),
    /// Additional data for sepa bank transfer payout method
    Sepa(Box<SepaBankTransferAdditionalData>),
    /// Additional data for pix bank transfer payout method
    Pix(Box<PixBankTransferAdditionalData>),
}

/// Masked payout method details for ach bank transfer payout method
#[derive(
    Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
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
    pub bank_country_code: Option<common_enums::CountryAlpha2>,

    /// Bank city
    #[schema(value_type = Option<String>, example = "California")]
    pub bank_city: Option<String>,
}

/// Masked payout method details for bacs bank transfer payout method
#[derive(
    Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
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
    pub bank_country_code: Option<common_enums::CountryAlpha2>,

    /// Bank city
    #[schema(value_type = Option<String>, example = "California")]
    pub bank_city: Option<String>,
}

/// Masked payout method details for sepa bank transfer payout method
#[derive(
    Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct SepaBankTransferAdditionalData {
    /// Partially masked international bank account number (iban) for SEPA
    #[schema(value_type = String, example = "DE8937******013000")]
    pub iban: MaskedIban,

    /// Bank name
    #[schema(value_type = Option<String>, example = "Deutsche Bank")]
    pub bank_name: Option<String>,

    /// Bank country code
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub bank_country_code: Option<common_enums::CountryAlpha2>,

    /// Bank city
    #[schema(value_type = Option<String>, example = "California")]
    pub bank_city: Option<String>,

    /// [8 / 11 digits] Bank Identifier Code (bic) / Swift Code - used in many countries for identifying a bank and it's branches
    #[schema(value_type = Option<String>, example = "HSBCGB2LXXX")]
    pub bic: Option<MaskedBic>,
}

/// Masked payout method details for pix bank transfer payout method
#[derive(
    Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
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

/// Masked payout method details for wallet payout method
#[derive(
    Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(untagged)]
pub enum WalletAdditionalData {
    /// Additional data for paypal wallet payout method
    Paypal(Box<PaypalAdditionalData>),
    /// Additional data for venmo wallet payout method
    Venmo(Box<VenmoAdditionalData>),
    /// Additional data for Apple pay decrypt wallet payout method
    ApplePayDecrypt(Box<ApplePayDecryptAdditionalData>),
}

/// Masked payout method details for paypal wallet payout method
#[derive(
    Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
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

/// Masked payout method details for venmo wallet payout method
#[derive(
    Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct VenmoAdditionalData {
    /// mobile number linked to venmo account
    #[schema(value_type = Option<String>, example = "******* 3349")]
    pub telephone_number: Option<MaskedPhoneNumber>,
}

/// Masked payout method details for Apple pay decrypt wallet payout method
#[derive(
    Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct ApplePayDecryptAdditionalData {
    /// Card expiry month
    #[schema(value_type = String, example = "01")]
    pub card_exp_month: Secret<String>,

    /// Card expiry year
    #[schema(value_type = String, example = "2026")]
    pub card_exp_year: Secret<String>,

    /// Card holder name
    #[schema(value_type = String, example = "John Doe")]
    pub card_holder_name: Option<Secret<String>>,
}

/// Masked payout method details for wallet payout method
#[derive(
    Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(untagged)]
pub enum BankRedirectAdditionalData {
    /// Additional data for interac bank redirect payout method
    Interac(Box<InteracAdditionalData>),
}

/// Masked payout method details for interac bank redirect payout method
#[derive(
    Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct InteracAdditionalData {
    /// Email linked with interac account
    #[schema(value_type = Option<String>, example = "john.doe@example.com")]
    pub email: Option<MaskedEmail>,
}

/// additional payout method details for passthrough payout method
#[derive(
    Eq, PartialEq, Clone, Debug, Deserialize, Serialize, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct PassthroughAddtionalData {
    /// Psp_token of the passthrough flow
    #[schema(value_type = String, example = "token_12345")]
    pub psp_token: MaskedPspToken,
    /// token_type of the passthrough flow
    #[schema(value_type = PaymentMethodType, example = "paypal")]
    pub token_type: common_enums::PaymentMethodType,
}
