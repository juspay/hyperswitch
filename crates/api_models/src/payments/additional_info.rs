use common_utils::new_type::{
    MaskedBankAccount, MaskedIban, MaskedRoutingNumber, MaskedSortCode, MaskedUpiVpaId,
};
use masking::Secret;
use smithy::SmithyModel;
use utoipa::ToSchema;

use crate::enums as api_enums;

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum BankDebitAdditionalData {
    #[smithy(value_type = "AchBankDebitAdditionalData")]
    Ach(Box<AchBankDebitAdditionalData>),
    #[smithy(value_type = "BacsBankDebitAdditionalData")]
    Bacs(Box<BacsBankDebitAdditionalData>),
    #[smithy(value_type = "BecsBankDebitAdditionalData")]
    Becs(Box<BecsBankDebitAdditionalData>),
    #[smithy(value_type = "SepaBankDebitAdditionalData")]
    Sepa(Box<SepaBankDebitAdditionalData>),
    SepaGuarenteedDebit(Box<SepaBankDebitAdditionalData>),
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AchBankDebitAdditionalData {
    /// Partially masked account number for ach bank debit payment
    #[schema(value_type = String, example = "0001****3456")]
    #[smithy(value_type = "String")]
    pub account_number: MaskedBankAccount,

    /// Partially masked routing number for ach bank debit payment
    #[schema(value_type = String, example = "110***000")]
    #[smithy(value_type = "String")]
    pub routing_number: MaskedRoutingNumber,

    /// Card holder's name
    #[schema(value_type = Option<String>, example = "John Doe")]
    #[smithy(value_type = "Option<String>")]
    pub card_holder_name: Option<Secret<String>>,

    /// Bank account's owner name
    #[schema(value_type = Option<String>, example = "John Doe")]
    #[smithy(value_type = "Option<String>")]
    pub bank_account_holder_name: Option<Secret<String>>,

    /// Name of the bank
    #[schema(value_type = Option<BankNames>, example = "ach")]
    #[smithy(value_type = "Option<BankNames>")]
    pub bank_name: Option<common_enums::BankNames>,

    /// Bank account type
    #[schema(value_type = Option<BankType>, example = "checking")]
    #[smithy(value_type = "Option<BankType>")]
    pub bank_type: Option<common_enums::BankType>,

    /// Bank holder entity type
    #[schema(value_type = Option<BankHolderType>, example = "personal")]
    #[smithy(value_type = "Option<BankHolderType>")]
    pub bank_holder_type: Option<common_enums::BankHolderType>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BacsBankDebitAdditionalData {
    /// Partially masked account number for Bacs payment method
    #[schema(value_type = String, example = "0001****3456")]
    #[smithy(value_type = "String")]
    pub account_number: MaskedBankAccount,

    /// Partially masked sort code for Bacs payment method
    #[schema(value_type = String, example = "108800")]
    #[smithy(value_type = "String")]
    pub sort_code: MaskedSortCode,

    /// Bank account's owner name
    #[schema(value_type = Option<String>, example = "John Doe")]
    #[smithy(value_type = "Option<String>")]
    pub bank_account_holder_name: Option<Secret<String>>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BecsBankDebitAdditionalData {
    /// Partially masked account number for Becs payment method
    #[schema(value_type = String, example = "0001****3456")]
    #[smithy(value_type = "String")]
    pub account_number: MaskedBankAccount,

    /// Bank-State-Branch (bsb) number
    #[schema(value_type = String, example = "000000")]
    #[smithy(value_type = "String")]
    pub bsb_number: Secret<String>,

    /// Bank account's owner name
    #[schema(value_type = Option<String>, example = "John Doe")]
    #[smithy(value_type = "Option<String>")]
    pub bank_account_holder_name: Option<Secret<String>>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct SepaBankDebitAdditionalData {
    /// Partially masked international bank account number (iban) for SEPA
    #[schema(value_type = String, example = "DE8937******013000")]
    #[smithy(value_type = "String")]
    pub iban: MaskedIban,

    /// Bank account's owner name
    #[schema(value_type = Option<String>, example = "John Doe")]
    #[smithy(value_type = "Option<String>")]
    pub bank_account_holder_name: Option<Secret<String>>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum BankRedirectDetails {
    #[smithy(value_type = "BancontactBankRedirectAdditionalData")]
    BancontactCard(Box<BancontactBankRedirectAdditionalData>),
    #[smithy(value_type = "BlikBankRedirectAdditionalData")]
    Blik(Box<BlikBankRedirectAdditionalData>),
    #[smithy(value_type = "GiropayBankRedirectAdditionalData")]
    Giropay(Box<GiropayBankRedirectAdditionalData>),
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BancontactBankRedirectAdditionalData {
    /// Last 4 digits of the card number
    #[schema(value_type = Option<String>, example = "4242")]
    #[smithy(value_type = "Option<String>")]
    pub last4: Option<String>,

    /// The card's expiry month
    #[schema(value_type = Option<String>, example = "12")]
    #[smithy(value_type = "Option<String>")]
    pub card_exp_month: Option<Secret<String>>,

    /// The card's expiry year
    #[schema(value_type = Option<String>, example = "24")]
    #[smithy(value_type = "Option<String>")]
    pub card_exp_year: Option<Secret<String>>,

    /// The card holder's name
    #[schema(value_type = Option<String>, example = "John Test")]
    #[smithy(value_type = "Option<String>")]
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct BlikBankRedirectAdditionalData {
    #[schema(value_type = Option<String>, example = "3GD9MO")]
    #[smithy(value_type = "Option<String>")]
    pub blik_code: Option<String>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GiropayBankRedirectAdditionalData {
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    /// Masked bank account bic code
    pub bic: Option<MaskedSortCode>,

    /// Partially masked international bank account number (iban) for SEPA
    #[schema(value_type = Option<String>)]
    #[smithy(value_type = "Option<String>")]
    pub iban: Option<MaskedIban>,

    /// Country for bank payment
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    #[smithy(value_type = "Option<CountryAlpha2>")]
    pub country: Option<api_enums::CountryAlpha2>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum BankTransferAdditionalData {
    #[smithy(nested_value_type)]
    Ach {},
    #[smithy(nested_value_type)]
    Sepa {},
    #[smithy(nested_value_type)]
    Bacs {},
    #[smithy(nested_value_type)]
    Multibanco {},
    #[smithy(nested_value_type)]
    Permata {},
    #[smithy(nested_value_type)]
    Bca {},
    #[smithy(nested_value_type)]
    BniVa {},
    #[smithy(nested_value_type)]
    BriVa {},
    #[smithy(nested_value_type)]
    CimbVa {},
    #[smithy(nested_value_type)]
    DanamonVa {},
    #[smithy(nested_value_type)]
    MandiriVa {},
    #[smithy(value_type = "PixBankTransferAdditionalData")]
    Pix(Box<PixBankTransferAdditionalData>),
    #[smithy(nested_value_type)]
    Pse {},
    #[smithy(value_type = "LocalBankTransferAdditionalData")]
    LocalBankTransfer(Box<LocalBankTransferAdditionalData>),
    #[smithy(nested_value_type)]
    InstantBankTransfer {},
    #[smithy(nested_value_type)]
    InstantBankTransferFinland {},
    #[smithy(nested_value_type)]
    InstantBankTransferPoland {},
    #[smithy(nested_value_type)]
    IndonesianBankTransfer {
        #[schema(value_type = Option<BankNames>, example = "bri")]
        #[smithy(value_type = "Option<BankNames>")]
        bank_name: Option<common_enums::BankNames>,
    },
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct PixBankTransferAdditionalData {
    /// Partially masked unique key for pix transfer
    #[schema(value_type = Option<String>, example = "a1f4102e ****** 6fa48899c1d1")]
    #[smithy(value_type = "Option<String>")]
    pub pix_key: Option<MaskedBankAccount>,

    /// Partially masked CPF - CPF is a Brazilian tax identification number
    #[schema(value_type = Option<String>, example = "**** 124689")]
    #[smithy(value_type = "Option<String>")]
    pub cpf: Option<MaskedBankAccount>,

    /// Partially masked CNPJ - CNPJ is a Brazilian company tax identification number
    #[schema(value_type = Option<String>, example = "**** 417312")]
    #[smithy(value_type = "Option<String>")]
    pub cnpj: Option<MaskedBankAccount>,

    /// Partially masked source bank account number
    #[schema(value_type = Option<String>, example = "********-****-4073-****-9fa964d08bc5")]
    #[smithy(value_type = "Option<String>")]
    pub source_bank_account_id: Option<MaskedBankAccount>,

    /// Partially masked destination bank account number _Deprecated: Will be removed in next stable release._
    #[schema(value_type = Option<String>, example = "********-****-460b-****-f23b4e71c97b", deprecated)]
    #[smithy(value_type = "Option<String>")]
    pub destination_bank_account_id: Option<MaskedBankAccount>,

    /// The expiration date and time for the Pix QR code in ISO 8601 format
    #[schema(value_type = Option<String>, example = "2025-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<String>")]
    pub expiry_date: Option<time::PrimitiveDateTime>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct LocalBankTransferAdditionalData {
    /// Partially masked bank code
    #[schema(value_type = Option<String>, example = "**** OA2312")]
    #[smithy(value_type = "Option<String>")]
    pub bank_code: Option<MaskedBankAccount>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum GiftCardAdditionalData {
    #[smithy(value_type = "String")]
    Givex(Box<GivexGiftCardAdditionalData>),
    #[smithy(nested_value_type)]
    PaySafeCard {},
    #[smithy(nested_value_type)]
    BhnCardNetwork {},
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct GivexGiftCardAdditionalData {
    /// Last 4 digits of the gift card number
    #[schema(value_type = String, example = "4242")]
    #[smithy(value_type = "String")]
    pub last4: Secret<String>,
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CardTokenAdditionalData {
    /// The card holder's name
    #[schema(value_type = String, example = "John Test")]
    #[smithy(value_type = "String")]
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum UpiAdditionalData {
    #[smithy(value_type = "UpiCollectAdditionalData")]
    UpiCollect(Box<UpiCollectAdditionalData>),
    #[schema(value_type = UpiIntentData)]
    #[smithy(value_type = "UpiIntentData")]
    UpiIntent(Box<super::UpiIntentData>),
    #[schema(value_type = UpiQrData)]
    UpiQr(Box<super::UpiQrData>),
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct UpiCollectAdditionalData {
    /// Masked VPA ID
    #[schema(value_type = Option<String>, example = "ab********@okhdfcbank")]
    #[smithy(value_type = "Option<String>")]
    pub vpa_id: Option<MaskedUpiVpaId>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct WalletAdditionalDataForCard {
    /// Last 4 digits of the card number
    #[smithy(value_type = "String")]
    pub last4: String,
    /// The information of the payment method
    #[smithy(value_type = "String")]
    pub card_network: String,
    /// The type of payment method
    #[serde(rename = "type")]
    #[smithy(value_type = "Option<String>")]
    pub card_type: Option<String>,
    /// The card's expiry month
    #[schema(value_type = Option<String>, example = "03")]
    pub card_exp_month: Option<Secret<String>>,
    /// The card's expiry year
    #[schema(value_type = Option<String>, example = "25")]
    pub card_exp_year: Option<Secret<String>>,
}
