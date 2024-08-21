use std::{
    collections::{HashMap, HashSet},
    fmt,
    num::NonZeroI64,
};

use cards::CardNumber;
use common_utils::{
    consts::default_payments_list_limit,
    crypto,
    ext_traits::{ConfigExt, Encode, ValueExt},
    hashing::HashedString,
    id_type,
    pii::{self, Email, EmailStrategy},
    types::{keymanager::ToEncryptable, MinorUnit, StringMajorUnit},
};
use error_stack::ResultExt;
use euclid::dssa::graph::euclid_graph_prelude::FxHashMap;
use masking::{ExposeInterface, PeekInterface, Secret, SwitchStrategy, WithType};
use router_derive::Setter;
use serde::{
    de::{self, Unexpected, Visitor},
    ser::Serializer,
    Deserialize, Deserializer, Serialize,
};
use strum::Display;
use time::{Date, PrimitiveDateTime};
use url::Url;
use utoipa::ToSchema;

use crate::{
    admin::{self, MerchantConnectorInfo},
    disputes, enums as api_enums,
    ephemeral_key::EphemeralKeyCreateResponse,
    mandates::RecurringDetails,
    refunds,
};

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BankDebitAdditionalData {
    Ach(AchBankDebitAdditionalData),
    Bacs(BacsBankDebitAdditionalData),
    Becs(BecsBankDebitAdditionalData),
    Sepa(SepaBankDebitAdditionalData),
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AchBankDebitAdditionalData {
    /// Partially masked account number for ach bank debit payment
    #[schema(value_type = String, example = "0001 **** 3456")]
    pub account_number: Secret<String>,
    /// Partially masked routing number for ach bank debit payment
    #[schema(value_type = String, example = "110 **** 000")]
    pub routing_number: Secret<String>,

    #[schema(value_type = String, example = "John Test")]
    pub card_holder_name: Option<Secret<String>>,

    #[schema(value_type = String, example = "John Doe")]
    pub bank_account_holder_name: Option<Secret<String>>,

    #[schema(value_type = String, example = "ACH")]
    pub bank_name: Option<common_enums::BankNames>,

    #[schema(value_type = String, example = "Checking")]
    pub bank_type: Option<common_enums::BankType>,

    #[schema(value_type = String, example = "Personal")]
    pub bank_holder_type: Option<common_enums::BankHolderType>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BacsBankDebitAdditionalData {
    /// Partially masked account number for Bacs payment method
    #[schema(value_type = String, example = "0001 **** 3456")]
    pub account_number: Secret<String>,
    /// Sort code for Bacs payment method
    #[schema(value_type = String, example = "108800")]
    pub sort_code: Secret<String>,
    /// holder name for bank debit
    #[schema(value_type = String, example = "A. Schneider")]
    pub bank_account_holder_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BecsBankDebitAdditionalData {
    /// Partially masked account number for Becs payment method
    #[schema(value_type = String, example = "0001 **** 3456")]
    pub account_number: Secret<String>,
    /// Bank-State-Branch (bsb) number
    #[schema(value_type = String, example = "000000")]
    pub bsb_number: Secret<String>,
    /// Owner name for bank debit
    #[schema(value_type = Option<String>, example = "A. Schneider")]
    pub bank_account_holder_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SepaBankDebitAdditionalData {
    /// Partially masked international bank account number (iban) for SEPA
    #[schema(value_type = String, example = "DE8937 ****** 013000")]
    pub iban: Secret<String>,
    /// Owner name for bank debit
    #[schema(value_type = String, example = "A. Schneider")]
    pub bank_account_holder_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub struct BankRedirectAdditionalData {
    pub bank_name: Option<common_enums::BankNames>,
    pub additional_details: Option<BankRedirectDetails>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub enum BankRedirectDetails {
    BancontactCard(BancontactBankRedirectAdditionalData),
    Blik(BlikBankRedirectAdditionalData),
    Giropay(GiropayBankRedirectAdditionalData),
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BancontactBankRedirectAdditionalData {
    /// Last 4 digits of the card number
    #[schema(value_type = Option<String>, example = "4242")]
    last4: Option<Secret<String>>,

    /// The card's expiry month
    #[schema(value_type = Option<String>, example = "12")]
    card_exp_month: Option<Secret<String>>,

    /// The card's expiry year
    #[schema(value_type = Option<String>, example = "24")]
    card_exp_year: Option<Secret<String>>,

    /// The card holder's name
    #[schema(value_type = Option<String>, example = "John Test")]
    card_holder_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BlikBankRedirectAdditionalData {
    #[schema(value_type = Option<String>, example = "3GD9MO")]
    blik_code: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GiropayBankRedirectAdditionalData {
    #[schema(value_type = Option<String>)]
    /// Masked bank account bic code
    bank_account_bic: Option<Secret<String>>,

    /// Partially masked international bank account number (iban) for SEPA
    #[schema(value_type = Option<String>)]
    bank_account_iban: Option<Secret<String>>,

    /// Country for bank payment
    #[schema(value_type = CountryAlpha2, example = "US")]
    country: Option<api_enums::CountryAlpha2>,
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
    Pix(PixBankTransferAdditionalData),
    Pse {},
    LocalBankTransfer(LocalBankTransferAdditionalData),
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PixBankTransferAdditionalData {
    /// Partially masked unique key for pix transfer
    #[schema(value_type = Option<String>, example = "a1f4102e ****** 6fa48899c1d1")]
    pub pix_key: Option<Secret<String>>,
    /// Partially masked CPF - CPF is a Brazilian tax identification number
    #[schema(value_type = Option<String>, example = "**** 124689")]
    pub cpf: Option<Secret<String>>,
    /// Partially masked CNPJ - CNPJ is a Brazilian company tax identification number
    #[schema(value_type = Option<String>, example = "**** 417312")]
    pub cnpj: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct LocalBankTransferAdditionalData {
    /// Partially masked bank code
    #[schema(value_type = Option<String>, example = "**** OA2312")]
    pub bank_code: Option<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GiftCardAdditionalData {
    Givex(GivexGiftCardAdditionalData),
    PaySafeCard {},
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GivexGiftCardAdditionalData {
    /// Partially masked gift card number
    #[schema(value_type = String, example = "4242 ****** 4242")]
    pub number: Secret<String>,

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
    UpiCollect(UpiCollectAdditionalData),
    UpiIntent(super::UpiIntentData),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UpiCollectAdditionalData {
    /// Masked VPA ID
    #[schema(value_type = Option<String>, example = "succ****@iata")]
    pub vpa_id: Option<Secret<String, pii::UpiVpaMaskingStrategy>>,
}
