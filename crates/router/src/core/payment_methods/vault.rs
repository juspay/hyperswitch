use common_utils::{
    crypto::{DecodeMessage, EncodeMessage, GcmAes256},
    ext_traits::BytesExt,
    generate_id_with_default_len,
};
use error_stack::{report, IntoReport, ResultExt};
use masking::PeekInterface;
use router_env::{instrument, tracing};
use scheduler::{types::process_data, utils as process_tracker_utils};

#[cfg(feature = "payouts")]
use crate::types::api::payouts;
use crate::{
    consts,
    core::errors::{self, CustomResult, RouterResult},
    db, logger, routes,
    routes::metrics,
    types::{
        api, domain,
        storage::{self, enums, ProcessTrackerExt},
    },
    utils::{self, StringExt},
};
const VAULT_SERVICE_NAME: &str = "CARD";

pub struct SupplementaryVaultData {
    pub customer_id: Option<String>,
    pub payment_method_id: Option<String>,
}

pub trait Vaultable: Sized {
    fn get_value1(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError>;
    fn get_value2(&self, _customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        Ok(String::new())
    }
    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError>;
}

impl Vaultable for api::Card {
    fn get_value1(&self, _customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value1 = api::TokenizedCardValue1 {
            card_number: self.card_number.peek().clone(),
            exp_year: self.card_exp_year.peek().clone(),
            exp_month: self.card_exp_month.peek().clone(),
            name_on_card: self
                .card_holder_name
                .as_ref()
                .map(|name| name.peek().clone()),
            nickname: None,
            card_last_four: None,
            card_token: None,
        };

        utils::Encode::<api::TokenizedCardValue1>::encode_to_string_of_json(&value1)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode card value1")
    }

    fn get_value2(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value2 = api::TokenizedCardValue2 {
            card_security_code: Some(self.card_cvc.peek().clone()),
            card_fingerprint: None,
            external_id: None,
            customer_id,
            payment_method_id: None,
        };

        utils::Encode::<api::TokenizedCardValue2>::encode_to_string_of_json(&value2)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode card value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: api::TokenizedCardValue1 = value1
            .parse_struct("TokenizedCardValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into card value1")?;

        let value2: api::TokenizedCardValue2 = value2
            .parse_struct("TokenizedCardValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into card value2")?;

        let card = Self {
            card_number: value1
                .card_number
                .try_into()
                .into_report()
                .change_context(errors::VaultError::ResponseDeserializationFailed)
                .attach_printable("Invalid card number format from the mock locker")?,
            card_exp_month: value1.exp_month.into(),
            card_exp_year: value1.exp_year.into(),
            card_holder_name: value1.name_on_card.map(masking::Secret::new),
            card_cvc: value2.card_security_code.unwrap_or_default().into(),
            card_issuer: None,
            card_network: None,
            bank_code: None,
            card_issuing_country: None,
            card_type: None,
            nick_name: value1.nickname.map(masking::Secret::new),
        };

        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: value2.payment_method_id,
        };

        Ok((card, supp_data))
    }
}

impl Vaultable for api_models::payments::BankTransferData {
    fn get_value1(&self, _customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value1 = api_models::payment_methods::TokenizedBankTransferValue1 {
            data: self.to_owned(),
        };

        utils::Encode::<api_models::payment_methods::TokenizedBankTransferValue1>::encode_to_string_of_json(&value1)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode bank transfer data")
    }

    fn get_value2(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value2 = api_models::payment_methods::TokenizedBankTransferValue2 { customer_id };

        utils::Encode::<api_models::payment_methods::TokenizedBankTransferValue2>::encode_to_string_of_json(&value2)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode bank transfer supplementary data")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: api_models::payment_methods::TokenizedBankTransferValue1 = value1
            .parse_struct("TokenizedBankTransferValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into bank transfer data")?;

        let value2: api_models::payment_methods::TokenizedBankTransferValue2 = value2
            .parse_struct("TokenizedBankTransferValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into supplementary bank transfer data")?;

        let bank_transfer_data = value1.data;

        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: None,
        };

        Ok((bank_transfer_data, supp_data))
    }
}

impl Vaultable for api::WalletData {
    fn get_value1(&self, _customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value1 = api::TokenizedWalletValue1 {
            data: self.to_owned(),
        };

        utils::Encode::<api::TokenizedWalletValue1>::encode_to_string_of_json(&value1)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode wallet data value1")
    }

    fn get_value2(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value2 = api::TokenizedWalletValue2 { customer_id };

        utils::Encode::<api::TokenizedWalletValue2>::encode_to_string_of_json(&value2)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode wallet data value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: api::TokenizedWalletValue1 = value1
            .parse_struct("TokenizedWalletValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into wallet data value1")?;

        let value2: api::TokenizedWalletValue2 = value2
            .parse_struct("TokenizedWalletValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into wallet data value2")?;

        let wallet = value1.data;

        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: None,
        };

        Ok((wallet, supp_data))
    }
}

impl Vaultable for api_models::payments::BankRedirectData {
    fn get_value1(&self, _customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value1 = api_models::payment_methods::TokenizedBankRedirectValue1 {
            data: self.to_owned(),
        };

        utils::Encode::<api_models::payment_methods::TokenizedBankRedirectValue1>::encode_to_string_of_json(&value1)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode bank redirect data")
    }

    fn get_value2(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value2 = api_models::payment_methods::TokenizedBankRedirectValue2 { customer_id };

        utils::Encode::<api_models::payment_methods::TokenizedBankRedirectValue2>::encode_to_string_of_json(&value2)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode bank redirect supplementary data")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: api_models::payment_methods::TokenizedBankRedirectValue1 = value1
            .parse_struct("TokenizedBankRedirectValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into bank redirect data")?;

        let value2: api_models::payment_methods::TokenizedBankRedirectValue2 = value2
            .parse_struct("TokenizedBankRedirectValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into supplementary bank redirect data")?;

        let bank_transfer_data = value1.data;

        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: None,
        };

        Ok((bank_transfer_data, supp_data))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum VaultPaymentMethod {
    Card(String),
    Wallet(String),
    BankTransfer(String),
    BankRedirect(String),
}

impl Vaultable for api::PaymentMethodData {
    fn get_value1(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value1 = match self {
            Self::Card(card) => VaultPaymentMethod::Card(card.get_value1(customer_id)?),
            Self::Wallet(wallet) => VaultPaymentMethod::Wallet(wallet.get_value1(customer_id)?),
            Self::BankTransfer(bank_transfer) => {
                VaultPaymentMethod::BankTransfer(bank_transfer.get_value1(customer_id)?)
            }
            Self::BankRedirect(bank_redirect) => {
                VaultPaymentMethod::BankRedirect(bank_redirect.get_value1(customer_id)?)
            }
            _ => Err(errors::VaultError::PaymentMethodNotSupported)
                .into_report()
                .attach_printable("Payment method not supported")?,
        };

        utils::Encode::<VaultPaymentMethod>::encode_to_string_of_json(&value1)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode payment method value1")
    }

    fn get_value2(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value2 = match self {
            Self::Card(card) => VaultPaymentMethod::Card(card.get_value2(customer_id)?),
            Self::Wallet(wallet) => VaultPaymentMethod::Wallet(wallet.get_value2(customer_id)?),
            Self::BankTransfer(bank_transfer) => {
                VaultPaymentMethod::BankTransfer(bank_transfer.get_value2(customer_id)?)
            }
            Self::BankRedirect(bank_redirect) => {
                VaultPaymentMethod::BankRedirect(bank_redirect.get_value2(customer_id)?)
            }
            _ => Err(errors::VaultError::PaymentMethodNotSupported)
                .into_report()
                .attach_printable("Payment method not supported")?,
        };

        utils::Encode::<VaultPaymentMethod>::encode_to_string_of_json(&value2)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode payment method value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: VaultPaymentMethod = value1
            .parse_struct("PaymentMethodValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into payment method value 1")?;

        let value2: VaultPaymentMethod = value2
            .parse_struct("PaymentMethodValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into payment method value 2")?;

        match (value1, value2) {
            (VaultPaymentMethod::Card(mvalue1), VaultPaymentMethod::Card(mvalue2)) => {
                let (card, supp_data) = api::Card::from_values(mvalue1, mvalue2)?;
                Ok((Self::Card(card), supp_data))
            }
            (VaultPaymentMethod::Wallet(mvalue1), VaultPaymentMethod::Wallet(mvalue2)) => {
                let (wallet, supp_data) = api::WalletData::from_values(mvalue1, mvalue2)?;
                Ok((Self::Wallet(wallet), supp_data))
            }
            (
                VaultPaymentMethod::BankTransfer(mvalue1),
                VaultPaymentMethod::BankTransfer(mvalue2),
            ) => {
                let (bank_transfer, supp_data) =
                    api_models::payments::BankTransferData::from_values(mvalue1, mvalue2)?;
                Ok((Self::BankTransfer(Box::new(bank_transfer)), supp_data))
            }
            (
                VaultPaymentMethod::BankRedirect(mvalue1),
                VaultPaymentMethod::BankRedirect(mvalue2),
            ) => {
                let (bank_redirect, supp_data) =
                    api_models::payments::BankRedirectData::from_values(mvalue1, mvalue2)?;
                Ok((Self::BankRedirect(bank_redirect), supp_data))
            }

            _ => Err(errors::VaultError::PaymentMethodNotSupported)
                .into_report()
                .attach_printable("Payment method not supported"),
        }
    }
}

#[cfg(feature = "payouts")]
impl Vaultable for api::CardPayout {
    fn get_value1(&self, _customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value1 = api::TokenizedCardValue1 {
            card_number: self.card_number.peek().clone(),
            exp_year: self.expiry_year.peek().clone(),
            exp_month: self.expiry_month.peek().clone(),
            name_on_card: self.card_holder_name.clone().map(|n| n.peek().to_string()),
            nickname: None,
            card_last_four: None,
            card_token: None,
        };

        utils::Encode::<api::TokenizedCardValue1>::encode_to_string_of_json(&value1)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode card value1")
    }

    fn get_value2(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value2 = api::TokenizedCardValue2 {
            card_security_code: None,
            card_fingerprint: None,
            external_id: None,
            customer_id,
            payment_method_id: None,
        };

        utils::Encode::<api::TokenizedCardValue2>::encode_to_string_of_json(&value2)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode card value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: api::TokenizedCardValue1 = value1
            .parse_struct("TokenizedCardValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into card value1")?;

        let value2: api::TokenizedCardValue2 = value2
            .parse_struct("TokenizedCardValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into card value2")?;

        let card = Self {
            card_number: value1
                .card_number
                .parse()
                .map_err(|_| errors::VaultError::FetchCardFailed)?,
            expiry_month: value1.exp_month.into(),
            expiry_year: value1.exp_year.into(),
            card_holder_name: value1.name_on_card.map(masking::Secret::new),
        };

        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: value2.payment_method_id,
        };

        Ok((card, supp_data))
    }
}

#[cfg(feature = "payouts")]
impl Vaultable for api::WalletPayout {
    fn get_value1(&self, _customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value1 = match self {
            Self::Paypal(paypal_data) => api::TokenizedWalletValue1 {
                data: api::WalletData::PaypalRedirect(api_models::payments::PaypalRedirection {
                    email: paypal_data.email.clone(),
                }),
            },
        };

        utils::Encode::<api::TokenizedWalletValue1>::encode_to_string_of_json(&value1)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode wallet value1")
    }

    fn get_value2(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value2 = api::TokenizedWalletValue2 { customer_id };

        utils::Encode::<api::TokenizedWalletValue2>::encode_to_string_of_json(&value2)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode wallet value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: api::TokenizedWalletValue1 = value1
            .parse_struct("TokenizedWalletValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into wallet value1")?;

        let value2: api::TokenizedWalletValue2 = value2
            .parse_struct("TokenizedWalletValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into wallet value2")?;

        let wallet = match value1.data {
            api::WalletData::PaypalRedirect(paypal_data) => {
                Self::Paypal(api_models::payouts::Paypal {
                    email: paypal_data.email,
                })
            }
            _ => Err(errors::VaultError::ResponseDeserializationFailed)?,
        };

        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: None,
        };

        Ok((wallet, supp_data))
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankSensitiveValues {
    pub bank_account_number: Option<masking::Secret<String>>,
    pub bank_routing_number: Option<masking::Secret<String>>,
    pub bic: Option<masking::Secret<String>>,
    pub bank_sort_code: Option<masking::Secret<String>>,
    pub iban: Option<masking::Secret<String>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankInsensitiveValues {
    pub customer_id: Option<String>,
    pub bank_name: Option<String>,
    pub bank_country_code: Option<api::enums::CountryAlpha2>,
    pub bank_city: Option<String>,
}

#[cfg(feature = "payouts")]
impl Vaultable for api::BankPayout {
    fn get_value1(&self, _customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let bank_sensitive_data = match self {
            Self::Ach(b) => TokenizedBankSensitiveValues {
                bank_account_number: Some(b.bank_account_number.clone()),
                bank_routing_number: Some(b.bank_routing_number.to_owned()),
                bic: None,
                bank_sort_code: None,
                iban: None,
            },
            Self::Bacs(b) => TokenizedBankSensitiveValues {
                bank_account_number: Some(b.bank_account_number.to_owned()),
                bank_routing_number: None,
                bic: None,
                bank_sort_code: Some(b.bank_sort_code.to_owned()),
                iban: None,
            },
            Self::Sepa(b) => TokenizedBankSensitiveValues {
                bank_account_number: None,
                bank_routing_number: None,
                bic: b.bic.to_owned(),
                bank_sort_code: None,
                iban: Some(b.iban.to_owned()),
            },
        };

        utils::Encode::<TokenizedBankSensitiveValues>::encode_to_string_of_json(
            &bank_sensitive_data,
        )
        .change_context(errors::VaultError::RequestEncodingFailed)
        .attach_printable("Failed to encode wallet data bank_sensitive_data")
    }

    fn get_value2(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let bank_insensitive_data = match self {
            Self::Ach(b) => TokenizedBankInsensitiveValues {
                customer_id,
                bank_name: b.bank_name.to_owned(),
                bank_country_code: b.bank_country_code.to_owned(),
                bank_city: b.bank_city.to_owned(),
            },
            Self::Bacs(b) => TokenizedBankInsensitiveValues {
                customer_id,
                bank_name: b.bank_name.to_owned(),
                bank_country_code: b.bank_country_code.to_owned(),
                bank_city: b.bank_city.to_owned(),
            },
            Self::Sepa(b) => TokenizedBankInsensitiveValues {
                customer_id,
                bank_name: b.bank_name.to_owned(),
                bank_country_code: b.bank_country_code.to_owned(),
                bank_city: b.bank_city.to_owned(),
            },
        };

        utils::Encode::<TokenizedBankInsensitiveValues>::encode_to_string_of_json(
            &bank_insensitive_data,
        )
        .change_context(errors::VaultError::RequestEncodingFailed)
        .attach_printable("Failed to encode wallet data bank_insensitive_data")
    }

    fn from_values(
        bank_sensitive_data: String,
        bank_insensitive_data: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let bank_sensitive_data: TokenizedBankSensitiveValues = bank_sensitive_data
            .parse_struct("TokenizedBankValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into bank data bank_sensitive_data")?;

        let bank_insensitive_data: TokenizedBankInsensitiveValues = bank_insensitive_data
            .parse_struct("TokenizedBankValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into wallet data bank_insensitive_data")?;

        let bank = match (
            // ACH + BACS
            bank_sensitive_data.bank_account_number.to_owned(),
            bank_sensitive_data.bank_routing_number.to_owned(), // ACH
            bank_sensitive_data.bank_sort_code.to_owned(),      // BACS
            // SEPA
            bank_sensitive_data.iban.to_owned(),
            bank_sensitive_data.bic,
        ) {
            (Some(ban), Some(brn), None, None, None) => Self::Ach(payouts::AchBankTransfer {
                bank_account_number: ban,
                bank_routing_number: brn,
                bank_name: bank_insensitive_data.bank_name,
                bank_country_code: bank_insensitive_data.bank_country_code,
                bank_city: bank_insensitive_data.bank_city,
            }),
            (Some(ban), None, Some(bsc), None, None) => Self::Bacs(payouts::BacsBankTransfer {
                bank_account_number: ban,
                bank_sort_code: bsc,
                bank_name: bank_insensitive_data.bank_name,
                bank_country_code: bank_insensitive_data.bank_country_code,
                bank_city: bank_insensitive_data.bank_city,
            }),
            (None, None, None, Some(iban), bic) => Self::Sepa(payouts::SepaBankTransfer {
                iban,
                bic,
                bank_name: bank_insensitive_data.bank_name,
                bank_country_code: bank_insensitive_data.bank_country_code,
                bank_city: bank_insensitive_data.bank_city,
            }),
            _ => Err(errors::VaultError::ResponseDeserializationFailed)?,
        };

        let supp_data = SupplementaryVaultData {
            customer_id: bank_insensitive_data.customer_id,
            payment_method_id: None,
        };

        Ok((bank, supp_data))
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum VaultPayoutMethod {
    Card(String),
    Bank(String),
    Wallet(String),
}

#[cfg(feature = "payouts")]
impl Vaultable for api::PayoutMethodData {
    fn get_value1(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value1 = match self {
            Self::Card(card) => VaultPayoutMethod::Card(card.get_value1(customer_id)?),
            Self::Bank(bank) => VaultPayoutMethod::Bank(bank.get_value1(customer_id)?),
            Self::Wallet(wallet) => VaultPayoutMethod::Wallet(wallet.get_value1(customer_id)?),
        };

        utils::Encode::<VaultPaymentMethod>::encode_to_string_of_json(&value1)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode payout method value1")
    }

    fn get_value2(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value2 = match self {
            Self::Card(card) => VaultPayoutMethod::Card(card.get_value2(customer_id)?),
            Self::Bank(bank) => VaultPayoutMethod::Bank(bank.get_value2(customer_id)?),
            Self::Wallet(wallet) => VaultPayoutMethod::Wallet(wallet.get_value2(customer_id)?),
        };

        utils::Encode::<VaultPaymentMethod>::encode_to_string_of_json(&value2)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode payout method value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: VaultPayoutMethod = value1
            .parse_struct("VaultMethodValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into vault method value 1")?;

        let value2: VaultPayoutMethod = value2
            .parse_struct("VaultMethodValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into vault method value 2")?;

        match (value1, value2) {
            (VaultPayoutMethod::Card(mvalue1), VaultPayoutMethod::Card(mvalue2)) => {
                let (card, supp_data) = api::CardPayout::from_values(mvalue1, mvalue2)?;
                Ok((Self::Card(card), supp_data))
            }
            (VaultPayoutMethod::Bank(mvalue1), VaultPayoutMethod::Bank(mvalue2)) => {
                let (bank, supp_data) = api::BankPayout::from_values(mvalue1, mvalue2)?;
                Ok((Self::Bank(bank), supp_data))
            }
            (VaultPayoutMethod::Wallet(mvalue1), VaultPayoutMethod::Wallet(mvalue2)) => {
                let (wallet, supp_data) = api::WalletPayout::from_values(mvalue1, mvalue2)?;
                Ok((Self::Wallet(wallet), supp_data))
            }
            _ => Err(errors::VaultError::PayoutMethodNotSupported)
                .into_report()
                .attach_printable("Payout method not supported"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MockTokenizeDBValue {
    pub value1: String,
    pub value2: String,
}

pub struct Vault;

impl Vault {
    #[instrument(skip_all)]
    pub async fn get_payment_method_data_from_locker(
        state: &routes::AppState,
        lookup_key: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<(Option<api::PaymentMethodData>, SupplementaryVaultData)> {
        let de_tokenize =
            get_tokenized_data(state, lookup_key, true, merchant_key_store.key.get_inner()).await?;
        let (payment_method, customer_id) =
            api::PaymentMethodData::from_values(de_tokenize.value1, de_tokenize.value2)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error parsing Payment Method from Values")?;

        Ok((Some(payment_method), customer_id))
    }

    #[instrument(skip_all)]
    pub async fn store_payment_method_data_in_locker(
        state: &routes::AppState,
        token_id: Option<String>,
        payment_method: &api::PaymentMethodData,
        customer_id: Option<String>,
        pm: enums::PaymentMethod,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<String> {
        let value1 = payment_method
            .get_value1(customer_id.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting Value1 for locker")?;

        let value2 = payment_method
            .get_value2(customer_id)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting Value12 for locker")?;

        let lookup_key = token_id.unwrap_or_else(|| generate_id_with_default_len("token"));

        let lookup_key = create_tokenize(
            state,
            value1,
            Some(value2),
            lookup_key,
            merchant_key_store.key.get_inner(),
        )
        .await?;
        add_delete_tokenized_data_task(&*state.store, &lookup_key, pm).await?;
        metrics::TOKENIZED_DATA_COUNT.add(&metrics::CONTEXT, 1, &[]);
        Ok(lookup_key)
    }

    #[cfg(feature = "payouts")]
    #[instrument(skip_all)]
    pub async fn get_payout_method_data_from_temporary_locker(
        state: &routes::AppState,
        lookup_key: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<(Option<api::PayoutMethodData>, SupplementaryVaultData)> {
        let de_tokenize =
            get_tokenized_data(state, lookup_key, true, merchant_key_store.key.get_inner()).await?;
        let (payout_method, supp_data) =
            api::PayoutMethodData::from_values(de_tokenize.value1, de_tokenize.value2)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error parsing Payout Method from Values")?;

        Ok((Some(payout_method), supp_data))
    }

    #[cfg(feature = "payouts")]
    #[instrument(skip_all)]
    pub async fn store_payout_method_data_in_locker(
        state: &routes::AppState,
        token_id: Option<String>,
        payout_method: &api::PayoutMethodData,
        customer_id: Option<String>,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<String> {
        let value1 = payout_method
            .get_value1(customer_id.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting Value1 for locker")?;

        let value2 = payout_method
            .get_value2(customer_id)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting Value2 for locker")?;

        let lookup_key =
            token_id.unwrap_or_else(|| generate_id_with_default_len("temporary_token"));

        let lookup_key = create_tokenize(
            state,
            value1,
            Some(value2),
            lookup_key,
            merchant_key_store.key.get_inner(),
        )
        .await?;
        // add_delete_tokenized_data_task(&*state.store, &lookup_key, pm).await?;
        // scheduler_metrics::TOKENIZED_DATA_COUNT.add(&metrics::CONTEXT, 1, &[]);
        Ok(lookup_key)
    }

    #[instrument(skip_all)]
    pub async fn delete_locker_payment_method_by_lookup_key(
        state: &routes::AppState,
        lookup_key: &Option<String>,
    ) {
        if let Some(lookup_key) = lookup_key {
            delete_tokenized_data(state, lookup_key)
                .await
                .map(|_| logger::info!("Card From locker deleted Successfully"))
                .map_err(|err| logger::error!("Error: Deleting Card From Redis Locker : {:?}", err))
                .ok();
        }
    }
}

//------------------------------------------------TokenizeService------------------------------------------------

#[inline(always)]
fn get_redis_locker_key(lookup_key: &str) -> String {
    format!("{}_{}", consts::LOCKER_REDIS_PREFIX, lookup_key)
}

#[instrument(skip(state, value1, value2))]
pub async fn create_tokenize(
    state: &routes::AppState,
    value1: String,
    value2: Option<String>,
    lookup_key: String,
    encryption_key: &masking::Secret<Vec<u8>>,
) -> RouterResult<String> {
    let redis_key = get_redis_locker_key(lookup_key.as_str());
    let func = || async {
        metrics::CREATED_TOKENIZED_CARD.add(&metrics::CONTEXT, 1, &[]);

        let payload_to_be_encrypted = api::TokenizePayloadRequest {
            value1: value1.clone(),
            value2: value2.clone().unwrap_or_default(),
            lookup_key: lookup_key.clone(),
            service_name: VAULT_SERVICE_NAME.to_string(),
        };

        let payload = utils::Encode::<api::TokenizePayloadRequest>::encode_to_string_of_json(
            &payload_to_be_encrypted,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let encrypted_payload = GcmAes256
            .encode_message(encryption_key.peek().as_ref(), payload.as_bytes())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encode redis temp locker data")?;

        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        redis_conn
            .set_key_if_not_exists_with_expiry(
                redis_key.as_str(),
                bytes::Bytes::from(encrypted_payload),
                Some(i64::from(consts::LOCKER_REDIS_EXPIRY_SECONDS)),
            )
            .await
            .map(|_| lookup_key.clone())
            .map_err(|err| {
                metrics::TEMP_LOCKER_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                err
            })
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error from redis locker")
    };

    match func().await {
        Ok(s) => {
            logger::info!(
                "Insert payload in redis locker successful with lookup key: {:?}",
                redis_key
            );
            Ok(s)
        }
        Err(err) => {
            logger::error!("Redis Temp locker Failed: {:?}", err);
            Err(err)
        }
    }
}

#[instrument(skip(state))]
pub async fn get_tokenized_data(
    state: &routes::AppState,
    lookup_key: &str,
    _should_get_value2: bool,
    encryption_key: &masking::Secret<Vec<u8>>,
) -> RouterResult<api::TokenizePayloadRequest> {
    let redis_key = get_redis_locker_key(lookup_key);
    let func = || async {
        metrics::GET_TOKENIZED_CARD.add(&metrics::CONTEXT, 1, &[]);

        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        let response = redis_conn.get_key::<bytes::Bytes>(redis_key.as_str()).await;

        match response {
            Ok(resp) => {
                let decrypted_payload = GcmAes256
                    .decode_message(
                        encryption_key.peek().as_ref(),
                        masking::Secret::new(resp.into()),
                    )
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to decode redis temp locker data")?;

                let get_response: api::TokenizePayloadRequest =
                    bytes::Bytes::from(decrypted_payload)
                        .parse_struct("TokenizePayloadRequest")
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Error getting TokenizePayloadRequest from tokenize response",
                        )?;

                Ok(get_response)
            }
            Err(err) => {
                metrics::TEMP_LOCKER_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                Err(err).change_context(errors::ApiErrorResponse::UnprocessableEntity {
                    message: "Token is invalid or expired".into(),
                })
            }
        }
    };

    match func().await {
        Ok(s) => {
            logger::info!(
                "Fetch payload in redis locker successful with lookup key: {:?}",
                redis_key
            );
            Ok(s)
        }
        Err(err) => {
            logger::error!("Redis Temp locker Failed: {:?}", err);
            Err(err)
        }
    }
}

#[instrument(skip(state))]
pub async fn delete_tokenized_data(state: &routes::AppState, lookup_key: &str) -> RouterResult<()> {
    let redis_key = get_redis_locker_key(lookup_key);
    let func = || async {
        metrics::DELETED_TOKENIZED_CARD.add(&metrics::CONTEXT, 1, &[]);

        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        let response = redis_conn.delete_key(redis_key.as_str()).await;

        match response {
            Ok(redis_interface::DelReply::KeyDeleted) => Ok(()),
            Ok(redis_interface::DelReply::KeyNotDeleted) => {
                Err(errors::ApiErrorResponse::InternalServerError)
                    .into_report()
                    .attach_printable("Token invalid or expired")
            }
            Err(err) => {
                metrics::TEMP_LOCKER_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                Err(errors::ApiErrorResponse::InternalServerError)
                    .into_report()
                    .attach_printable_lazy(|| {
                        format!("Failed to delete from redis locker: {err:?}")
                    })
            }
        }
    };
    match func().await {
        Ok(s) => {
            logger::info!(
                "Delete payload in redis locker successful with lookup key: {:?}",
                redis_key
            );
            Ok(s)
        }
        Err(err) => {
            logger::error!("Redis Temp locker Failed: {:?}", err);
            Err(err)
        }
    }
}

// ********************************************** PROCESS TRACKER **********************************************

pub async fn add_delete_tokenized_data_task(
    db: &dyn db::StorageInterface,
    lookup_key: &str,
    pm: enums::PaymentMethod,
) -> RouterResult<()> {
    let runner = scheduler::types::PTRunner::DeleteTokenizeDataWorkflow;
    let process_tracker_id = format!("{runner}_{lookup_key}");
    let task = runner.to_string();
    let tag = ["BASILISK-V3"];
    let tracking_data = storage::TokenizeCoreWorkflow {
        lookup_key: lookup_key.to_owned(),
        pm,
    };
    let schedule_time = get_delete_tokenize_schedule_time(db, &pm, 0)
        .await
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .into_report()
        .attach_printable("Failed to obtain initial process tracker schedule time")?;

    let process_tracker_entry =
        <storage::ProcessTracker as ProcessTrackerExt>::make_process_tracker_new(
            process_tracker_id,
            &task,
            runner,
            tag,
            tracking_data,
            schedule_time,
        )
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to construct delete tokenized data process tracker task")?;

    let response = db.insert_process(process_tracker_entry).await;
    response.map(|_| ()).or_else(|err| {
        if err.current_context().is_db_unique_violation() {
            Ok(())
        } else {
            Err(report!(errors::ApiErrorResponse::InternalServerError))
        }
    })
}

pub async fn start_tokenize_data_workflow(
    state: &routes::AppState,
    tokenize_tracker: &storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    let delete_tokenize_data = serde_json::from_value::<storage::TokenizeCoreWorkflow>(
        tokenize_tracker.tracking_data.clone(),
    )
    .into_report()
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable_lazy(|| {
        format!(
            "unable to convert into DeleteTokenizeByTokenRequest {:?}",
            tokenize_tracker.tracking_data
        )
    })?;

    match delete_tokenized_data(state, &delete_tokenize_data.lookup_key).await {
        Ok(()) => {
            logger::info!("Card From locker deleted Successfully");
            //mark task as finished
            let id = tokenize_tracker.id.clone();
            tokenize_tracker
                .clone()
                .finish_with_status(db.as_scheduler(), format!("COMPLETED_BY_PT_{id}"))
                .await?;
        }
        Err(err) => {
            logger::error!("Err: Deleting Card From Locker : {:?}", err);
            retry_delete_tokenize(db, &delete_tokenize_data.pm, tokenize_tracker.to_owned())
                .await?;
            metrics::RETRIED_DELETE_DATA_COUNT.add(&metrics::CONTEXT, 1, &[]);
        }
    }
    Ok(())
}

pub async fn get_delete_tokenize_schedule_time(
    db: &dyn db::StorageInterface,
    pm: &enums::PaymentMethod,
    retry_count: i32,
) -> Option<time::PrimitiveDateTime> {
    let redis_mapping = db::get_and_deserialize_key(
        db,
        &format!("pt_mapping_delete_{pm}_tokenize_data"),
        "PaymentMethodsPTMapping",
    )
    .await;
    let mapping = match redis_mapping {
        Ok(x) => x,
        Err(err) => {
            logger::info!("Redis Mapping Error: {}", err);
            process_data::PaymentMethodsPTMapping::default()
        }
    };
    let time_delta = process_tracker_utils::get_pm_schedule_time(mapping, pm, retry_count + 1);

    process_tracker_utils::get_time_from_delta(time_delta)
}

pub async fn retry_delete_tokenize(
    db: &dyn db::StorageInterface,
    pm: &enums::PaymentMethod,
    pt: storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let schedule_time = get_delete_tokenize_schedule_time(db, pm, pt.retry_count).await;

    match schedule_time {
        Some(s_time) => {
            let retry_schedule = pt.retry(db.as_scheduler(), s_time).await;
            metrics::TASKS_RESET_COUNT.add(
                &metrics::CONTEXT,
                1,
                &[metrics::request::add_attributes(
                    "flow",
                    "DeleteTokenizeData",
                )],
            );
            retry_schedule
        }
        None => {
            pt.finish_with_status(db.as_scheduler(), "RETRIES_EXCEEDED".to_string())
                .await
        }
    }
}

// Fallback logic of old temp locker needs to be removed later
