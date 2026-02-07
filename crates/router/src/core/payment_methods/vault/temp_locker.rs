use common_enums::PaymentMethodType;
use common_utils::{
    crypto::{DecodeMessage, EncodeMessage, GcmAes256},
    ext_traits::{BytesExt, Encode},
    generate_id_with_default_len, id_type,
    pii::Email,
};
use error_stack::ResultExt;
use masking::PeekInterface;
use router_env::{instrument, tracing};
use std::convert::TryFrom;

#[cfg(feature = "payouts")]
use crate::types::api::payouts;
use crate::{
    consts,
    core::errors::{self, CustomResult, RouterResult},
    logger,
    routes::{self, metrics},
    types::{api, domain, storage::enums},
    utils::StringExt,
};

mod process_tracker;

pub(crate) struct SupplementaryVaultData {
    pub customer_id: Option<id_type::CustomerId>,
    pub payment_method_id: Option<String>,
}

trait TempVaultable: Sized {
    fn get_value1(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError>;
    fn get_value2(
        &self,
        _customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        Ok(String::new())
    }
    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError>;
}

impl TempVaultable for domain::Card {
    fn get_value1(
        &self,
        _customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value1 = domain::TokenizedCardValue1 {
            card_number: self.card_number.peek().clone(),
            exp_year: self.card_exp_year.peek().clone(),
            exp_month: self.card_exp_month.peek().clone(),
            nickname: self.nick_name.as_ref().map(|name| name.peek().clone()),
            card_last_four: None,
            card_token: None,
            card_holder_name: self.card_holder_name.clone(),
        };

        value1
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode card value1")
    }

    fn get_value2(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value2 = domain::TokenizedCardValue2 {
            card_security_code: Some(self.card_cvc.peek().clone()),
            card_fingerprint: None,
            external_id: None,
            customer_id,
            payment_method_id: None,
        };

        value2
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode card value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: domain::TokenizedCardValue1 = value1
            .parse_struct("TokenizedCardValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into card value1")?;

        let value2: domain::TokenizedCardValue2 = value2
            .parse_struct("TokenizedCardValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into card value2")?;

        let card = Self {
            card_number: cards::CardNumber::try_from(value1.card_number)
                .change_context(errors::VaultError::ResponseDeserializationFailed)
                .attach_printable("Invalid card number format from the mock locker")?,
            card_exp_month: value1.exp_month.into(),
            card_exp_year: value1.exp_year.into(),
            card_cvc: value2.card_security_code.unwrap_or_default().into(),
            card_issuer: None,
            card_network: None,
            bank_code: None,
            card_issuing_country: None,
            card_issuing_country_code: None,
            card_type: None,
            nick_name: value1.nickname.map(masking::Secret::new),
            card_holder_name: value1.card_holder_name,
            co_badged_card_data: None,
        };

        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: value2.payment_method_id,
        };

        Ok((card, supp_data))
    }
}

impl TempVaultable for domain::BankTransferData {
    fn get_value1(
        &self,
        _customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value1 = domain::TokenizedBankTransferValue1 {
            data: self.to_owned(),
        };

        value1
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode bank transfer data")
    }

    fn get_value2(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value2 = domain::TokenizedBankTransferValue2 { customer_id };

        value2
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode bank transfer supplementary data")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: domain::TokenizedBankTransferValue1 = value1
            .parse_struct("TokenizedBankTransferValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into bank transfer data")?;

        let value2: domain::TokenizedBankTransferValue2 = value2
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

impl TempVaultable for domain::WalletData {
    fn get_value1(
        &self,
        _customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value1 = domain::TokenizedWalletValue1 {
            data: self.to_owned(),
        };

        value1
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode wallet data value1")
    }

    fn get_value2(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value2 = domain::TokenizedWalletValue2 { customer_id };

        value2
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode wallet data value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: domain::TokenizedWalletValue1 = value1
            .parse_struct("TokenizedWalletValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into wallet data value1")?;

        let value2: domain::TokenizedWalletValue2 = value2
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

impl TempVaultable for domain::BankRedirectData {
    fn get_value1(
        &self,
        _customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value1 = domain::TokenizedBankRedirectValue1 {
            data: self.to_owned(),
        };

        value1
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode bank redirect data")
    }

    fn get_value2(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value2 = domain::TokenizedBankRedirectValue2 { customer_id };

        value2
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode bank redirect supplementary data")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: domain::TokenizedBankRedirectValue1 = value1
            .parse_struct("TokenizedBankRedirectValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into bank redirect data")?;

        let value2: domain::TokenizedBankRedirectValue2 = value2
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

impl TempVaultable for domain::BankDebitData {
    fn get_value1(
        &self,
        _customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value1 = domain::TokenizedBankDebitValue1 {
            data: self.to_owned(),
        };

        value1
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode bank debit data")
    }

    fn get_value2(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value2 = domain::TokenizedBankDebitValue2 { customer_id };

        value2
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode bank debit supplementary data")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: domain::TokenizedBankDebitValue1 = value1
            .parse_struct("TokenizedBankDebitValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into bank debit data")?;

        let value2: domain::TokenizedBankDebitValue2 = value2
            .parse_struct("TokenizedBankDebitValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into supplementary bank debit data")?;

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
    BankDebit(String),
}

impl TempVaultable for domain::PaymentMethodData {
    fn get_value1(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value1 = match self {
            Self::Card(card) => VaultPaymentMethod::Card(card.get_value1(customer_id)?),
            Self::Wallet(wallet) => VaultPaymentMethod::Wallet(wallet.get_value1(customer_id)?),
            Self::BankTransfer(bank_transfer) => {
                VaultPaymentMethod::BankTransfer(bank_transfer.get_value1(customer_id)?)
            }
            Self::BankRedirect(bank_redirect) => {
                VaultPaymentMethod::BankRedirect(bank_redirect.get_value1(customer_id)?)
            }
            Self::BankDebit(bank_debit) => {
                VaultPaymentMethod::BankDebit(bank_debit.get_value1(customer_id)?)
            }
            _ => Err(errors::VaultError::PaymentMethodNotSupported)
                .attach_printable("Payment method not supported")?,
        };

        value1
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode payment method value1")
    }

    fn get_value2(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value2 = match self {
            Self::Card(card) => VaultPaymentMethod::Card(card.get_value2(customer_id)?),
            Self::Wallet(wallet) => VaultPaymentMethod::Wallet(wallet.get_value2(customer_id)?),
            Self::BankTransfer(bank_transfer) => {
                VaultPaymentMethod::BankTransfer(bank_transfer.get_value2(customer_id)?)
            }
            Self::BankRedirect(bank_redirect) => {
                VaultPaymentMethod::BankRedirect(bank_redirect.get_value2(customer_id)?)
            }
            Self::BankDebit(bank_debit) => {
                VaultPaymentMethod::BankDebit(bank_debit.get_value2(customer_id)?)
            }
            _ => Err(errors::VaultError::PaymentMethodNotSupported)
                .attach_printable("Payment method not supported")?,
        };

        value2
            .encode_to_string_of_json()
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
                let (card, supp_data) = domain::Card::from_values(mvalue1, mvalue2)?;
                Ok((Self::Card(card), supp_data))
            }
            (VaultPaymentMethod::Wallet(mvalue1), VaultPaymentMethod::Wallet(mvalue2)) => {
                let (wallet, supp_data) = domain::WalletData::from_values(mvalue1, mvalue2)?;
                Ok((Self::Wallet(wallet), supp_data))
            }
            (
                VaultPaymentMethod::BankTransfer(mvalue1),
                VaultPaymentMethod::BankTransfer(mvalue2),
            ) => {
                let (bank_transfer, supp_data) =
                    domain::BankTransferData::from_values(mvalue1, mvalue2)?;
                Ok((Self::BankTransfer(Box::new(bank_transfer)), supp_data))
            }
            (
                VaultPaymentMethod::BankRedirect(mvalue1),
                VaultPaymentMethod::BankRedirect(mvalue2),
            ) => {
                let (bank_redirect, supp_data) =
                    domain::BankRedirectData::from_values(mvalue1, mvalue2)?;
                Ok((Self::BankRedirect(bank_redirect), supp_data))
            }
            (VaultPaymentMethod::BankDebit(mvalue1), VaultPaymentMethod::BankDebit(mvalue2)) => {
                let (bank_debit, supp_data) = domain::BankDebitData::from_values(mvalue1, mvalue2)?;
                Ok((Self::BankDebit(bank_debit), supp_data))
            }

            _ => Err(errors::VaultError::PaymentMethodNotSupported)
                .attach_printable("Payment method not supported"),
        }
    }
}

#[cfg(feature = "payouts")]
impl TempVaultable for api::CardPayout {
    fn get_value1(
        &self,
        _customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value1 = api::TokenizedCardValue1 {
            card_number: self.card_number.peek().clone(),
            exp_year: self.expiry_year.peek().clone(),
            exp_month: self.expiry_month.peek().clone(),
            name_on_card: self.card_holder_name.clone().map(|n| n.peek().to_string()),
            nickname: None,
            card_last_four: None,
            card_token: None,
            card_network: self.card_network.clone(),
        };

        value1
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode card value1")
    }

    fn get_value2(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value2 = api::TokenizedCardValue2 {
            card_security_code: None,
            card_fingerprint: None,
            external_id: None,
            customer_id,
            payment_method_id: None,
        };

        value2
            .encode_to_string_of_json()
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
            card_network: value1.card_network,
        };

        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: value2.payment_method_id,
        };

        Ok((card, supp_data))
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TokenizedWalletSensitiveValues {
    pub email: Option<Email>,
    pub telephone_number: Option<masking::Secret<String>>,
    pub wallet_id: Option<masking::Secret<String>>,
    pub wallet_type: PaymentMethodType,
    pub dpan: Option<cards::CardNumber>,
    pub expiry_month: Option<masking::Secret<String>>,
    pub expiry_year: Option<masking::Secret<String>>,
    pub card_holder_name: Option<masking::Secret<String>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TokenizedWalletInsensitiveValues {
    pub customer_id: Option<id_type::CustomerId>,
    pub card_network: Option<common_enums::CardNetwork>,
}

#[cfg(feature = "payouts")]
impl TempVaultable for api::WalletPayout {
    fn get_value1(
        &self,
        _customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value1 = match self {
            Self::Paypal(paypal_data) => TokenizedWalletSensitiveValues {
                email: paypal_data.email.clone(),
                telephone_number: paypal_data.telephone_number.clone(),
                wallet_id: paypal_data.paypal_id.clone(),
                wallet_type: PaymentMethodType::Paypal,
                dpan: None,
                expiry_month: None,
                expiry_year: None,
                card_holder_name: None,
            },
            Self::Venmo(venmo_data) => TokenizedWalletSensitiveValues {
                email: None,
                telephone_number: venmo_data.telephone_number.clone(),
                wallet_id: None,
                wallet_type: PaymentMethodType::Venmo,
                dpan: None,
                expiry_month: None,
                expiry_year: None,
                card_holder_name: None,
            },
            Self::ApplePayDecrypt(apple_pay_decrypt_data) => TokenizedWalletSensitiveValues {
                email: None,
                telephone_number: None,
                wallet_id: None,
                wallet_type: PaymentMethodType::ApplePay,
                dpan: Some(apple_pay_decrypt_data.dpan.clone()),
                expiry_month: Some(apple_pay_decrypt_data.expiry_month.clone()),
                expiry_year: Some(apple_pay_decrypt_data.expiry_year.clone()),
                card_holder_name: apple_pay_decrypt_data.card_holder_name.clone(),
            },
        };

        value1
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode wallet data - TokenizedWalletSensitiveValues")
    }

    fn get_value2(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value2 = match self {
            Self::Paypal(_paypal_data) => TokenizedWalletInsensitiveValues {
                customer_id,
                card_network: None,
            },
            Self::Venmo(_venmo_data) => TokenizedWalletInsensitiveValues {
                customer_id,
                card_network: None,
            },
            Self::ApplePayDecrypt(apple_pay_decrypt_data) => TokenizedWalletInsensitiveValues {
                customer_id,
                card_network: apple_pay_decrypt_data.card_network.clone(),
            },
        };

        value2
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode data - TokenizedWalletInsensitiveValues")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: TokenizedWalletSensitiveValues = value1
            .parse_struct("TokenizedWalletSensitiveValues")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into wallet data wallet_sensitive_data")?;

        let value2: TokenizedWalletInsensitiveValues = value2
            .parse_struct("TokenizedWalletInsensitiveValues")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into wallet data wallet_insensitive_data")?;

        let wallet = match value1.wallet_type {
            PaymentMethodType::Paypal => Self::Paypal(api_models::payouts::Paypal {
                email: value1.email,
                telephone_number: value1.telephone_number,
                paypal_id: value1.wallet_id,
            }),
            PaymentMethodType::Venmo => Self::Venmo(api_models::payouts::Venmo {
                telephone_number: value1.telephone_number,
            }),
            PaymentMethodType::ApplePay => {
                match (value1.dpan, value1.expiry_month, value1.expiry_year) {
                    (Some(dpan), Some(expiry_month), Some(expiry_year)) => {
                        Self::ApplePayDecrypt(api_models::payouts::ApplePayDecrypt {
                            dpan,
                            expiry_month,
                            expiry_year,
                            card_holder_name: value1.card_holder_name,
                            card_network: value2.card_network,
                        })
                    }
                    _ => Err(errors::VaultError::ResponseDeserializationFailed)?,
                }
            }
            _ => Err(errors::VaultError::PayoutMethodNotSupported)?,
        };
        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: None,
        };

        Ok((wallet, supp_data))
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TokenizedBankSensitiveValues {
    pub bank_account_number: Option<masking::Secret<String>>,
    pub bank_routing_number: Option<masking::Secret<String>>,
    pub bic: Option<masking::Secret<String>>,
    pub bank_sort_code: Option<masking::Secret<String>>,
    pub iban: Option<masking::Secret<String>>,
    pub pix_key: Option<masking::Secret<String>>,
    pub tax_id: Option<masking::Secret<String>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TokenizedBankInsensitiveValues {
    pub customer_id: Option<id_type::CustomerId>,
    pub bank_name: Option<String>,
    pub bank_country_code: Option<api::enums::CountryAlpha2>,
    pub bank_city: Option<String>,
    pub bank_branch: Option<String>,
}

#[cfg(feature = "payouts")]
impl TempVaultable for api::BankPayout {
    fn get_value1(
        &self,
        _customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let bank_sensitive_data = match self {
            Self::Ach(b) => TokenizedBankSensitiveValues {
                bank_account_number: Some(b.bank_account_number.clone()),
                bank_routing_number: Some(b.bank_routing_number.to_owned()),
                bic: None,
                bank_sort_code: None,
                iban: None,
                pix_key: None,
                tax_id: None,
            },
            Self::Bacs(b) => TokenizedBankSensitiveValues {
                bank_account_number: Some(b.bank_account_number.to_owned()),
                bank_routing_number: None,
                bic: None,
                bank_sort_code: Some(b.bank_sort_code.to_owned()),
                iban: None,
                pix_key: None,
                tax_id: None,
            },
            Self::Sepa(b) => TokenizedBankSensitiveValues {
                bank_account_number: None,
                bank_routing_number: None,
                bic: b.bic.to_owned(),
                bank_sort_code: None,
                iban: Some(b.iban.to_owned()),
                pix_key: None,
                tax_id: None,
            },
            Self::Pix(bank_details) => TokenizedBankSensitiveValues {
                bank_account_number: Some(bank_details.bank_account_number.to_owned()),
                bank_routing_number: None,
                bic: None,
                bank_sort_code: None,
                iban: None,
                pix_key: Some(bank_details.pix_key.to_owned()),
                tax_id: bank_details.tax_id.to_owned(),
            },
        };

        bank_sensitive_data
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode data - bank_sensitive_data")
    }

    fn get_value2(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let bank_insensitive_data = match self {
            Self::Ach(b) => TokenizedBankInsensitiveValues {
                customer_id,
                bank_name: b.bank_name.to_owned(),
                bank_country_code: b.bank_country_code.to_owned(),
                bank_city: b.bank_city.to_owned(),
                bank_branch: None,
            },
            Self::Bacs(b) => TokenizedBankInsensitiveValues {
                customer_id,
                bank_name: b.bank_name.to_owned(),
                bank_country_code: b.bank_country_code.to_owned(),
                bank_city: b.bank_city.to_owned(),
                bank_branch: None,
            },
            Self::Sepa(bank_details) => TokenizedBankInsensitiveValues {
                customer_id,
                bank_name: bank_details.bank_name.to_owned(),
                bank_country_code: bank_details.bank_country_code.to_owned(),
                bank_city: bank_details.bank_city.to_owned(),
                bank_branch: None,
            },
            Self::Pix(bank_details) => TokenizedBankInsensitiveValues {
                customer_id,
                bank_name: bank_details.bank_name.to_owned(),
                bank_country_code: None,
                bank_city: None,
                bank_branch: bank_details.bank_branch.to_owned(),
            },
        };

        bank_insensitive_data
            .encode_to_string_of_json()
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
            // ACH + BACS + PIX
            bank_sensitive_data.bank_account_number.to_owned(),
            bank_sensitive_data.bank_routing_number.to_owned(), // ACH
            bank_sensitive_data.bank_sort_code.to_owned(),      // BACS
            // SEPA
            bank_sensitive_data.iban.to_owned(),
            bank_sensitive_data.bic,
            // PIX
            bank_sensitive_data.pix_key,
            bank_sensitive_data.tax_id,
        ) {
            (Some(ban), Some(brn), None, None, None, None, None) => {
                Self::Ach(payouts::AchBankTransfer {
                    bank_account_number: ban,
                    bank_routing_number: brn,
                    bank_name: bank_insensitive_data.bank_name,
                    bank_country_code: bank_insensitive_data.bank_country_code,
                    bank_city: bank_insensitive_data.bank_city,
                })
            }
            (Some(ban), None, Some(bsc), None, None, None, None) => {
                Self::Bacs(payouts::BacsBankTransfer {
                    bank_account_number: ban,
                    bank_sort_code: bsc,
                    bank_name: bank_insensitive_data.bank_name,
                    bank_country_code: bank_insensitive_data.bank_country_code,
                    bank_city: bank_insensitive_data.bank_city,
                })
            }
            (None, None, None, Some(iban), bic, None, None) => {
                Self::Sepa(payouts::SepaBankTransfer {
                    iban,
                    bic,
                    bank_name: bank_insensitive_data.bank_name,
                    bank_country_code: bank_insensitive_data.bank_country_code,
                    bank_city: bank_insensitive_data.bank_city,
                })
            }
            (Some(ban), None, None, None, None, Some(pix_key), tax_id) => {
                Self::Pix(payouts::PixBankTransfer {
                    bank_account_number: ban,
                    bank_branch: bank_insensitive_data.bank_branch,
                    bank_name: bank_insensitive_data.bank_name,
                    pix_key,
                    tax_id,
                })
            }
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
    BankRedirect(String),
    Passthrough(String),
}

#[cfg(feature = "payouts")]
impl TempVaultable for api::PayoutMethodData {
    fn get_value1(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value1 = match self {
            Self::Card(card) => VaultPayoutMethod::Card(card.get_value1(customer_id)?),
            Self::Bank(bank) => VaultPayoutMethod::Bank(bank.get_value1(customer_id)?),
            Self::Wallet(wallet) => VaultPayoutMethod::Wallet(wallet.get_value1(customer_id)?),
            Self::BankRedirect(bank_redirect) => {
                VaultPayoutMethod::BankRedirect(bank_redirect.get_value1(customer_id)?)
            }
            Self::Passthrough(passthrough) => {
                VaultPayoutMethod::Passthrough(passthrough.get_value1(customer_id)?)
            }
        };

        value1
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode payout method value1")
    }

    fn get_value2(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value2 = match self {
            Self::Card(card) => VaultPayoutMethod::Card(card.get_value2(customer_id)?),
            Self::Bank(bank) => VaultPayoutMethod::Bank(bank.get_value2(customer_id)?),
            Self::Wallet(wallet) => VaultPayoutMethod::Wallet(wallet.get_value2(customer_id)?),
            Self::BankRedirect(bank_redirect) => {
                VaultPayoutMethod::BankRedirect(bank_redirect.get_value2(customer_id)?)
            }
            Self::Passthrough(passthrough) => {
                VaultPayoutMethod::Passthrough(passthrough.get_value2(customer_id)?)
            }
        };

        value2
            .encode_to_string_of_json()
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
            (
                VaultPayoutMethod::BankRedirect(mvalue1),
                VaultPayoutMethod::BankRedirect(mvalue2),
            ) => {
                let (bank_redirect, supp_data) =
                    api::BankRedirectPayout::from_values(mvalue1, mvalue2)?;
                Ok((Self::BankRedirect(bank_redirect), supp_data))
            }
            (VaultPayoutMethod::Passthrough(mvalue1), VaultPayoutMethod::Passthrough(mvalue2)) => {
                let (passthrough, supp_data) =
                    api::PassthroughPayout::from_values(mvalue1, mvalue2)?;
                Ok((Self::Passthrough(passthrough), supp_data))
            }
            _ => Err(errors::VaultError::PayoutMethodNotSupported)
                .attach_printable("Payout method not supported"),
        }
    }
}

#[cfg(feature = "payouts")]
impl TempVaultable for api::BankRedirectPayout {
    fn get_value1(
        &self,
        _customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value1 = match self {
            Self::Interac(interac_data) => TokenizedBankRedirectSensitiveValues {
                email: interac_data.email.clone(),
                bank_redirect_type: PaymentMethodType::Interac,
            },
        };

        value1
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable(
                "Failed to encode bank redirect data - TokenizedBankRedirectSensitiveValues",
            )
    }

    fn get_value2(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value2 = TokenizedBankRedirectInsensitiveValues { customer_id };

        value2
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode wallet data value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: TokenizedBankRedirectSensitiveValues = value1
            .parse_struct("TokenizedBankRedirectSensitiveValues")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into wallet data value1")?;

        let value2: TokenizedBankRedirectInsensitiveValues = value2
            .parse_struct("TokenizedBankRedirectInsensitiveValues")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into wallet data value2")?;

        let bank_redirect = match value1.bank_redirect_type {
            PaymentMethodType::Interac => Self::Interac(api_models::payouts::Interac {
                email: value1.email,
            }),
            _ => Err(errors::VaultError::PayoutMethodNotSupported)
                .attach_printable("Payout method not supported")?,
        };

        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: None,
        };

        Ok((bank_redirect, supp_data))
    }
}

#[cfg(feature = "payouts")]
impl TempVaultable for api::PassthroughPayout {
    fn get_value1(
        &self,
        _customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value1 = TokenizedPassthroughSensitiveValues {
            psp_token: self.psp_token.clone(),
        };

        value1
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable(
                "Failed to encode passthrough data - TokenizedPassthroughSensitiveValues",
            )
    }

    fn get_value2(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        let value2 = TokenizedPassthroughInsensitiveValues {
            customer_id,
            token_type: self.token_type,
        };

        value2
            .encode_to_string_of_json()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode passthrough data value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: TokenizedPassthroughSensitiveValues = value1
            .parse_struct("TokenizedPassthroughSensitiveValues")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into connector token data value1")?;

        let value2: TokenizedPassthroughInsensitiveValues = value2
            .parse_struct("TokenizedPassthroughInsensitiveValues")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into connector token data value2")?;

        let passthrough = Self {
            psp_token: value1.psp_token,
            token_type: value2.token_type,
        };

        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: None,
        };

        Ok((passthrough, supp_data))
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TokenizedBankRedirectSensitiveValues {
    pub email: Email,
    pub bank_redirect_type: PaymentMethodType,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TokenizedBankRedirectInsensitiveValues {
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TokenizedPassthroughSensitiveValues {
    pub psp_token: masking::Secret<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TokenizedPassthroughInsensitiveValues {
    pub customer_id: Option<id_type::CustomerId>,
    pub token_type: PaymentMethodType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct MockTokenizeDBValue {
    pub value1: String,
    pub value2: String,
}

pub(crate) struct TempLocker;

impl TempLocker {
    #[instrument(skip_all)]
    pub(crate) async fn get_payment_method_data_from_temp_locker(
        state: &routes::SessionState,
        lookup_key: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<(Option<domain::PaymentMethodData>, SupplementaryVaultData)> {
        let de_tokenize =
            get_tokenized_data(state, lookup_key, true, merchant_key_store.key.get_inner()).await?;
        let (payment_method, customer_id) =
            domain::PaymentMethodData::from_values(de_tokenize.value1, de_tokenize.value2)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error parsing Payment Method from Values")?;

        Ok((Some(payment_method), customer_id))
    }

    #[instrument(skip_all)]
    pub(crate) async fn store_payment_method_data_in_temp_locker(
        state: &routes::SessionState,
        token_id: Option<String>,
        payment_method: &domain::PaymentMethodData,
        customer_id: Option<id_type::CustomerId>,
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

        let lookup_key = create_tokenize_without_configurable_expiry(
            state,
            value1,
            Some(value2),
            lookup_key,
            merchant_key_store.key.get_inner(),
        )
        .await?;
        process_tracker::add_delete_tokenized_data_task(
            &*state.store,
            &lookup_key,
            pm,
            state.conf.application_source,
        )
        .await?;
        metrics::TOKENIZED_DATA_COUNT.add(1, &[]);
        Ok(lookup_key)
    }

    #[cfg(feature = "payouts")]
    #[instrument(skip_all)]
    pub(crate) async fn get_payout_method_data_from_temporary_locker(
        state: &routes::SessionState,
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
    pub(crate) async fn store_payout_method_data_in_temp_locker(
        state: &routes::SessionState,
        token_id: Option<String>,
        payout_method: &api::PayoutMethodData,
        customer_id: Option<id_type::CustomerId>,
        merchant_key_store: &domain::MerchantKeyStore,
        intent_fulfillment_time: Option<i64>,
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

        let lookup_key = create_tokenize_with_configurable_expiry(
            state,
            value1,
            Some(value2),
            lookup_key,
            merchant_key_store.key.get_inner(),
            intent_fulfillment_time,
        )
        .await?;
        // add_delete_tokenized_data_task(&*state.store, &lookup_key, pm).await?;
        // scheduler_metrics::TOKENIZED_DATA_COUNT.add(1, &[]);
        Ok(lookup_key)
    }

    #[instrument(skip_all)]
    pub(crate) async fn delete_temp_locker_payment_method_by_lookup_key(
        state: &routes::SessionState,
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

const VAULT_SERVICE_NAME: &str = "CARD";

#[inline(always)]
fn get_redis_locker_key(lookup_key: &str) -> String {
    format!("{}_{}", consts::LOCKER_REDIS_PREFIX, lookup_key)
}

#[instrument(skip(state, value1, value2))]
pub(crate) async fn create_tokenize_without_configurable_expiry(
    state: &routes::SessionState,
    value1: String,
    value2: Option<String>,
    lookup_key: String,
    encryption_key: &masking::Secret<Vec<u8>>,
) -> RouterResult<String> {
    create_tokenize(state, value1, value2, lookup_key, encryption_key, None).await
}

#[instrument(skip(state, value1, value2))]
pub(crate) async fn create_tokenize_with_configurable_expiry(
    state: &routes::SessionState,
    value1: String,
    value2: Option<String>,
    lookup_key: String,
    encryption_key: &masking::Secret<Vec<u8>>,
    expiry_time: Option<i64>,
) -> RouterResult<String> {
    create_tokenize(
        state,
        value1,
        value2,
        lookup_key,
        encryption_key,
        expiry_time,
    )
    .await
}

#[instrument(skip(state, value1, value2))]
async fn create_tokenize(
    state: &routes::SessionState,
    value1: String,
    value2: Option<String>,
    lookup_key: String,
    encryption_key: &masking::Secret<Vec<u8>>,
    expiry_time: Option<i64>,
) -> RouterResult<String> {
    let redis_key = get_redis_locker_key(lookup_key.as_str());
    let func = || async {
        metrics::CREATED_TOKENIZED_CARD.add(1, &[]);

        let payload_to_be_encrypted = api::TokenizePayloadRequest {
            value1: value1.clone(),
            value2: value2.clone().unwrap_or_default(),
            lookup_key: lookup_key.clone(),
            service_name: VAULT_SERVICE_NAME.to_string(),
        };

        let payload = payload_to_be_encrypted
            .encode_to_string_of_json()
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
                &redis_key.as_str().into(),
                bytes::Bytes::from(encrypted_payload),
                expiry_time.or(Some(i64::from(consts::LOCKER_REDIS_EXPIRY_SECONDS))),
            )
            .await
            .map(|_| lookup_key.clone())
            .inspect_err(|error| {
                metrics::TEMP_LOCKER_FAILURES.add(1, &[]);
                logger::error!(?error, "Failed to store tokenized data in Redis");
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
pub(crate) async fn get_tokenized_data(
    state: &routes::SessionState,
    lookup_key: &str,
    _should_get_value2: bool,
    encryption_key: &masking::Secret<Vec<u8>>,
) -> RouterResult<api::TokenizePayloadRequest> {
    let redis_key = get_redis_locker_key(lookup_key);
    let func = || async {
        metrics::GET_TOKENIZED_CARD.add(1, &[]);

        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        let response = redis_conn
            .get_key::<bytes::Bytes>(&redis_key.as_str().into())
            .await;

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
                metrics::TEMP_LOCKER_FAILURES.add(1, &[]);
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
pub(crate) async fn delete_tokenized_data(
    state: &routes::SessionState,
    lookup_key: &str,
) -> RouterResult<()> {
    let redis_key = get_redis_locker_key(lookup_key);
    let func = || async {
        metrics::DELETED_TOKENIZED_CARD.add(1, &[]);

        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        let response = redis_conn.delete_key(&redis_key.as_str().into()).await;

        match response {
            Ok(redis_interface::DelReply::KeyDeleted) => Ok(()),
            Ok(redis_interface::DelReply::KeyNotDeleted) => {
                Err(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Token invalid or expired")
            }
            Err(err) => {
                metrics::TEMP_LOCKER_FAILURES.add(1, &[]);
                Err(errors::ApiErrorResponse::InternalServerError).attach_printable_lazy(|| {
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

// ----------------------------------------------------------------------------
