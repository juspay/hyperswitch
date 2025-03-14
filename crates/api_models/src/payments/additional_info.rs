use common_utils::new_type::{
    MaskedBankAccount, MaskedIban, MaskedRoutingNumber, MaskedSortCode, MaskedUpiVpaId,
};
use masking::Secret;
use utoipa::ToSchema;

use crate::enums as api_enums;

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BankDebitAdditionalData {
    Ach(Box<AchBankDebitAdditionalData>),
    Bacs(Box<BacsBankDebitAdditionalData>),
    Becs(Box<BecsBankDebitAdditionalData>),
    Sepa(Box<SepaBankDebitAdditionalData>),
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AchBankDebitAdditionalData {
    /// Partially masked account number for ach bank debit payment
    #[schema(value_type = String, example = "0001****3456")]
    pub account_number: MaskedBankAccount,

    /// Partially masked routing number for ach bank debit payment
    #[schema(value_type = String, example = "110***000")]
    pub routing_number: MaskedRoutingNumber,

    /// Card holder's name
    #[schema(value_type = Option<String>, example = "John Doe")]
    pub card_holder_name: Option<Secret<String>>,

    /// Bank account's owner name
    #[schema(value_type = Option<String>, example = "John Doe")]
    pub bank_account_holder_name: Option<Secret<String>>,

    /// Name of the bank
    #[schema(value_type = Option<BankNames>, example = "ach")]
    pub bank_name: Option<common_enums::BankNames>,

    /// Bank account type
    #[schema(value_type = Option<BankType>, example = "checking")]
    pub bank_type: Option<common_enums::BankType>,

    /// Bank holder entity type
    #[schema(value_type = Option<BankHolderType>, example = "personal")]
    pub bank_holder_type: Option<common_enums::BankHolderType>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BacsBankDebitAdditionalData {
    /// Partially masked account number for Bacs payment method
    #[schema(value_type = String, example = "0001****3456")]
    pub account_number: MaskedBankAccount,

    /// Partially masked sort code for Bacs payment method
    #[schema(value_type = String, example = "108800")]
    pub sort_code: MaskedSortCode,

    /// Bank account's owner name
    #[schema(value_type = Option<String>, example = "John Doe")]
    pub bank_account_holder_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BecsBankDebitAdditionalData {
    /// Partially masked account number for Becs payment method
    #[schema(value_type = String, example = "0001****3456")]
    pub account_number: MaskedBankAccount,

    /// Bank-State-Branch (bsb) number
    #[schema(value_type = String, example = "000000")]
    pub bsb_number: Secret<String>,

    /// Bank account's owner name
    #[schema(value_type = Option<String>, example = "John Doe")]
    pub bank_account_holder_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SepaBankDebitAdditionalData {
    /// Partially masked international bank account number (iban) for SEPA
    #[schema(value_type = String, example = "DE8937******013000")]
    pub iban: MaskedIban,

    /// Bank account's owner name
    #[schema(value_type = Option<String>, example = "John Doe")]
    pub bank_account_holder_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub enum BankRedirectDetails {
    BancontactCard(Box<BancontactBankRedirectAdditionalData>),
    Blik(Box<BlikBankRedirectAdditionalData>),
    Giropay(Box<GiropayBankRedirectAdditionalData>),
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BancontactBankRedirectAdditionalData {
    /// Last 4 digits of the card number
    #[schema(value_type = Option<String>, example = "4242")]
    pub last4: Option<String>,

    /// The card's expiry month
    #[schema(value_type = Option<String>, example = "12")]
    pub card_exp_month: Option<Secret<String>>,

    /// The card's expiry year
    #[schema(value_type = Option<String>, example = "24")]
    pub card_exp_year: Option<Secret<String>>,

    /// The card holder's name
    #[schema(value_type = Option<String>, example = "John Test")]
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BlikBankRedirectAdditionalData {
    #[schema(value_type = Option<String>, example = "3GD9MO")]
    pub blik_code: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GiropayBankRedirectAdditionalData {
    #[schema(value_type = Option<String>)]
    /// Masked bank account bic code
    pub bic: Option<MaskedSortCode>,

    /// Partially masked international bank account number (iban) for SEPA
    #[schema(value_type = Option<String>)]
    pub iban: Option<MaskedIban>,

    /// Country for bank payment
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub country: Option<api_enums::CountryAlpha2>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BankTransferAdditionalData {
    Ach {},
    Sepa {},
    Bacs {},
    Multibanco {},
    Permata {},
    Bca {},
    BniVa {},
    BriVa {},
    CimbVa {},
    DanamonVa {},
    MandiriVa {},
    Pix(Box<PixBankTransferAdditionalData>),
    Pse {},
    LocalBankTransfer(Box<LocalBankTransferAdditionalData>),
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PixBankTransferAdditionalData {
    /// Partially masked unique key for pix transfer
    #[schema(value_type = Option<String>, example = "a1f4102e ****** 6fa48899c1d1")]
    pub pix_key: Option<MaskedBankAccount>,

    /// Partially masked CPF - CPF is a Brazilian tax identification number
    #[schema(value_type = Option<String>, example = "**** 124689")]
    pub cpf: Option<MaskedBankAccount>,

    /// Partially masked CNPJ - CNPJ is a Brazilian company tax identification number
    #[schema(value_type = Option<String>, example = "**** 417312")]
    pub cnpj: Option<MaskedBankAccount>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct LocalBankTransferAdditionalData {
    /// Partially masked bank code
    #[schema(value_type = Option<String>, example = "**** OA2312")]
    pub bank_code: Option<MaskedBankAccount>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GiftCardAdditionalData {
    Givex(Box<GivexGiftCardAdditionalData>),
    PaySafeCard {},
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GivexGiftCardAdditionalData {
    /// Last 4 digits of the gift card number
    #[schema(value_type = String, example = "4242")]
    pub last4: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CardTokenAdditionalData {
    /// The card holder's name
    #[schema(value_type = String, example = "John Test")]
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UpiAdditionalData {
    UpiCollect(Box<UpiCollectAdditionalData>),
    #[schema(value_type = UpiIntentData)]
    UpiIntent(Box<super::UpiIntentData>),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct UpiCollectAdditionalData {
    /// Masked VPA ID
    #[schema(value_type = Option<String>, example = "ab********@okhdfcbank")]
    pub vpa_id: Option<MaskedUpiVpaId>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct WalletAdditionalDataForCard {
    /// Last 4 digits of the card number
    pub last4: String,
    /// The information of the payment method
    pub card_network: String,
    /// The type of payment method
    #[serde(rename = "type")]
    pub card_type: Option<String>,
}
