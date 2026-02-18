use api_models::payment_methods;
#[cfg(feature = "v2")]
use common_utils::{crypto::Encryptable, errors::CustomResult, ext_traits::OptionExt};
#[cfg(feature = "v2")]
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{errors, payment_method_data};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PaymentMethodVaultingData {
    Card(payment_methods::CardDetail),
    NetworkToken(payment_method_data::NetworkTokenDetails),
    CardNumber(cards::CardNumber),
    #[cfg(feature = "v1")]
    BankDebit(payment_methods::BankDebitDetail),
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum FingerprintData {
    Card(FingerprintCardData),
    NetworkToken(FingerprintNetworkTokenData),
    CardNumber(cards::CardNumber),
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct FingerprintCardData {
    card_number: cards::CardNumber,
    card_exp_month: masking::Secret<String>,
    card_exp_year: masking::Secret<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct FingerprintNetworkTokenData {
    network_token: cards::NetworkToken,
    network_token_exp_month: masking::Secret<String>,
    network_token_exp_year: masking::Secret<String>,
}

impl PaymentMethodVaultingData {
    #[cfg(feature = "v2")]
    pub fn get_card(&self) -> Option<&payment_methods::CardDetail> {
        match self {
            Self::Card(card) => Some(card),
            Self::NetworkToken(_) => None,
            Self::CardNumber(_) => None,
        }
    }

    #[cfg(feature = "v2")]
    pub fn set_card_cvc(&mut self, card_cvc: masking::Secret<String>) {
        match self {
            Self::Card(card_details) => {
                card_details.card_cvc = Some(card_cvc);
            }
            Self::NetworkToken(_) => {}
            Self::CardNumber(_) => {}
        }
    }

    #[cfg(feature = "v2")]
    pub fn convert_to_raw_payment_method_data(
        &self,
    ) -> Option<payment_methods::RawPaymentMethodData> {
        match self {
            Self::Card(card) => Some(payment_methods::RawPaymentMethodData::Card(card.clone())),
            // Raw payment methods data is not available for network tokens
            Self::NetworkToken(network_token) => None,
            // When it is card number populated_payment_methods_data_and_get_payment_method_vaulting_data
            // will be called which will populated the payment methods data for card number and convert it to type CardDetail
            Self::CardNumber(card_number) => None,
        }
    }

    #[cfg(feature = "v2")]
    pub fn populated_payment_methods_data_and_get_payment_method_vaulting_data(
        &self,
        payment_methods_data_optional: Option<&Encryptable<payment_methods::PaymentMethodsData>>,
    ) -> CustomResult<Self, errors::api_error_response::ApiErrorResponse> {
        match self {
            Self::Card(card_details) => {
                let payment_methods_data = payment_methods_data_optional
                    .get_required_value("payment methods data")
                    .change_context(
                            errors::api_error_response::ApiErrorResponse::InternalServerError,
                        )
                    .attach_printable("failed to get payment methods data for payment method vaulting data type card number")?;

                let payment_methods_data = payment_methods_data_optional
                    .get_required_value("payment methods data")
                    .change_context(
                            errors::api_error_response::ApiErrorResponse::InternalServerError,
                        )
                    .attach_printable("failed to get payment methods data for payment method vaulting data type card number")?;

                let card_detail = Self::populated_payment_methods_data_for_payment_method_vaulting_data_card_number(
                        &card_details.card_number,
                        card_details.card_cvc.clone(),
                        payment_methods_data,
                    )?;

                Ok(Self::Card(card_detail))
            }
            Self::NetworkToken(_) => Ok(self.clone()),
            Self::CardNumber(card_number) => {
                let payment_methods_data = payment_methods_data_optional
                    .get_required_value("payment methods data")
                    .change_context(
                            errors::api_error_response::ApiErrorResponse::InternalServerError,
                        )
                    .attach_printable("failed to get payment methods data for payment method vaulting data type card number")?;

                let card_detail = Self::populated_payment_methods_data_for_payment_method_vaulting_data_card_number(
                        card_number,
                        None,
                        payment_methods_data,
                    )?;

                Ok(Self::Card(card_detail))
            }
        }
    }

    #[cfg(feature = "v2")]
    pub fn populated_payment_methods_data_for_payment_method_vaulting_data_card_number(
        card_number: &cards::CardNumber,
        card_cvc: Option<masking::Secret<String>>,
        payment_methods_data: &Encryptable<payment_methods::PaymentMethodsData>,
    ) -> CustomResult<payment_methods::CardDetail, errors::api_error_response::ApiErrorResponse>
    {
        let stored_card_metadata = payment_methods_data
            .clone()
            .into_inner()
            .get_card_details()
            .get_required_value("card payment methods data")
            .change_context(errors::api_error_response::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to get stored card payment methods details")?;

        let card_with_details = payment_method_data::CardNumberWithStoredDetails::new(
            card_number.clone(),
            card_cvc.clone(),
            stored_card_metadata.into(),
        );

        payment_methods::CardDetail::try_from(card_with_details)
            .change_context(errors::api_error_response::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Failed to create card details for payment method vaulting data type card number ",
            )
    }

    #[cfg(feature = "v2")]
    pub fn get_payment_methods_data(&self) -> payment_method_data::PaymentMethodsData {
        match self {
            Self::Card(card) => payment_method_data::PaymentMethodsData::Card(
                payment_method_data::CardDetailsPaymentMethod::from(card.clone()),
            ),
            Self::NetworkToken(network_token) => {
                payment_method_data::PaymentMethodsData::NetworkToken(
                    payment_method_data::NetworkTokenDetailsPaymentMethod::from(
                        network_token.clone(),
                    ),
                )
            }
            Self::CardNumber(_card_number) => payment_method_data::PaymentMethodsData::Card(
                payment_method_data::CardDetailsPaymentMethod {
                    last4_digits: None,
                    issuer_country: None,
                    #[cfg(feature = "v1")]
                    issuer_country_code: None,
                    expiry_month: None,
                    expiry_year: None,
                    nick_name: None,
                    card_holder_name: None,
                    card_isin: None,
                    card_issuer: None,
                    card_network: None,
                    card_type: None,
                    saved_to_locker: false,
                    #[cfg(feature = "v1")]
                    co_badged_card_data: None,
                },
            ),
        }
    }

    #[cfg(feature = "v2")]
    pub fn to_fingerprint_data(&self) -> FingerprintData {
        match self {
            Self::Card(card) => FingerprintData::Card(FingerprintCardData {
                card_number: card.card_number.clone(),
                card_exp_month: card.card_exp_month.clone(),
                card_exp_year: card.card_exp_year.clone(),
            }),
            Self::NetworkToken(nt) => FingerprintData::NetworkToken(FingerprintNetworkTokenData {
                network_token: nt.network_token.clone(),
                network_token_exp_month: nt.network_token_exp_month.clone(),
                network_token_exp_year: nt.network_token_exp_year.clone(),
            }),
            Self::CardNumber(card_number) => FingerprintData::CardNumber(card_number.clone()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PaymentMethodCustomVaultingData {
    CardData(CardCustomData),
    NetworkTokenData(NetworkTokenCustomData),
}

impl Default for PaymentMethodCustomVaultingData {
    fn default() -> Self {
        Self::CardData(CardCustomData::default())
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct CardCustomData {
    pub card_number: Option<cards::CardNumber>,
    pub card_exp_month: Option<masking::Secret<String>>,
    pub card_exp_year: Option<masking::Secret<String>>,
    pub card_cvc: Option<masking::Secret<String>>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct NetworkTokenCustomData {
    pub network_token: Option<cards::NetworkToken>,
    pub network_token_exp_month: Option<masking::Secret<String>>,
    pub network_token_exp_year: Option<masking::Secret<String>>,
    pub cryptogram: Option<masking::Secret<String>>,
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V1VaultEntityId {
    merchant_id: common_utils::id_type::MerchantId,
    customer_id: common_utils::id_type::CustomerId,
}

#[cfg(feature = "v1")]
impl V1VaultEntityId {
    pub fn new(
        merchant_id: common_utils::id_type::MerchantId,
        customer_id: common_utils::id_type::CustomerId,
    ) -> Self {
        Self {
            merchant_id,
            customer_id,
        }
    }

    pub fn get_string_repr(&self) -> String {
        format!(
            "{}_{}",
            self.merchant_id.get_string_repr(),
            self.customer_id.get_string_repr()
        )
    }
}

#[cfg(feature = "v1")]
impl Serialize for V1VaultEntityId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.get_string_repr())
    }
}

#[cfg(feature = "v1")]
impl<'de> Deserialize<'de> for V1VaultEntityId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.splitn(2, '_').collect();

        let merchant_part = parts.first().ok_or_else(|| {
            serde::de::Error::custom(
                "Invalid V1VaultEntityId format: expected 'merchant_id_customer_id'",
            )
        })?;

        let customer_part = parts.get(1).ok_or_else(|| {
            serde::de::Error::custom(
                "Invalid V1VaultEntityId format: expected 'merchant_id_customer_id'",
            )
        })?;

        Ok(Self {
            merchant_id: common_utils::id_type::MerchantId::wrap((*merchant_part).to_string())
                .map_err(serde::de::Error::custom)?,
            customer_id: common_utils::id_type::CustomerId::wrap((*customer_part).to_string())
                .map_err(serde::de::Error::custom)?,
        })
    }
}

#[cfg(feature = "v2")]
pub trait VaultingDataInterface {
    fn get_vaulting_data_key(&self) -> String;
}

#[cfg(feature = "v2")]
impl VaultingDataInterface for PaymentMethodVaultingData {
    fn get_vaulting_data_key(&self) -> String {
        match &self {
            Self::Card(card) => card.card_number.to_string(),
            Self::NetworkToken(network_token) => network_token.network_token.to_string(),
            Self::CardNumber(card_number) => card_number.to_string(),
        }
    }
}
#[cfg(feature = "v2")]
impl VaultingDataInterface for FingerprintData {
    fn get_vaulting_data_key(&self) -> String {
        match self {
            Self::Card(card) => card.card_number.to_string(),
            Self::NetworkToken(network_token) => network_token.network_token.to_string(),
            Self::CardNumber(card_number) => card_number.to_string(),
        }
    }
}

#[cfg(feature = "v2")]
impl TryFrom<payment_methods::PaymentMethodCreateData> for PaymentMethodVaultingData {
    type Error = error_stack::Report<errors::api_error_response::ApiErrorResponse>;
    fn try_from(item: payment_methods::PaymentMethodCreateData) -> Result<Self, Self::Error> {
        match item {
            payment_methods::PaymentMethodCreateData::Card(card) => {
                Ok(Self::Card(payment_methods::CardDetail {
                    card_cvc: None, // card cvc should not be used for vaulting
                    ..card
                }))
            }
            payment_methods::PaymentMethodCreateData::ProxyCard(card) => Err(
                errors::api_error_response::ApiErrorResponse::UnprocessableEntity {
                    message: "Proxy Card for PaymentMethodCreateData".to_string(),
                }
                .into(),
            ),
        }
    }
}

#[cfg(feature = "v1")]
impl From<payment_methods::PaymentMethodCreateData> for PaymentMethodVaultingData {
    fn from(item: payment_methods::PaymentMethodCreateData) -> Self {
        match item {
            payment_methods::PaymentMethodCreateData::Card(card) => {
                Self::Card(payment_methods::CardDetail {
                    card_cvc: None, // card cvc should not be used for vaulting
                    ..card
                })
            }
            payment_methods::PaymentMethodCreateData::BankDebit(bank_debit_detail) => {
                Self::BankDebit(bank_debit_detail)
            }
        }
    }
}

impl TryFrom<PaymentMethodVaultingData> for PaymentMethodCustomVaultingData {
    type Error = error_stack::Report<errors::api_error_response::ApiErrorResponse>;

    fn try_from(item: PaymentMethodVaultingData) -> Result<Self, Self::Error> {
        match item {
            PaymentMethodVaultingData::Card(card_data) => Ok(Self::CardData(CardCustomData {
                card_number: Some(card_data.card_number),
                card_exp_month: Some(card_data.card_exp_month),
                card_exp_year: Some(card_data.card_exp_year),
                card_cvc: card_data.card_cvc,
            })),
            PaymentMethodVaultingData::NetworkToken(network_token_data) => {
                Ok(Self::NetworkTokenData(NetworkTokenCustomData {
                    network_token: Some(network_token_data.network_token),
                    network_token_exp_month: Some(network_token_data.network_token_exp_month),
                    network_token_exp_year: Some(network_token_data.network_token_exp_year),
                    cryptogram: network_token_data.cryptogram,
                }))
            }
            PaymentMethodVaultingData::CardNumber(card_number_data) => {
                Ok(Self::CardData(CardCustomData {
                    card_number: Some(card_number_data),
                    card_exp_month: None,
                    card_exp_year: None,
                    card_cvc: None,
                }))
            }
            #[cfg(feature = "v1")]
            PaymentMethodVaultingData::BankDebit(_) => Err(
                errors::api_error_response::ApiErrorResponse::NotImplemented {
                    message: errors::api_error_response::NotImplementedMessage::Reason(
                        "PaymentMethodCustomVaultingData not implemented for BankDebit".to_string(),
                    ),
                },
            )?,
        }
    }
}
