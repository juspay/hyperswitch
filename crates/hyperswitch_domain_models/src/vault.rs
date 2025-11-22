use api_models::payment_methods;
use serde::{Deserialize, Serialize};

#[cfg(feature = "v2")]
use crate::errors;
use crate::payment_method_data;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PaymentMethodVaultingData {
    Card(payment_methods::CardDetail),
    NetworkToken(payment_method_data::NetworkTokenDetails),
    CardNumber(cards::CardNumber),
}

impl PaymentMethodVaultingData {
    pub fn get_card(&self) -> Option<&payment_methods::CardDetail> {
        match self {
            Self::Card(card) => Some(card),
            Self::NetworkToken(_) => None,
            Self::CardNumber(_) => None,
        }
    }
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

pub trait VaultingDataInterface {
    fn get_vaulting_data_key(&self) -> String;
}

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
impl TryFrom<payment_methods::PaymentMethodCreateData> for PaymentMethodVaultingData {
    type Error = error_stack::Report<errors::api_error_response::ApiErrorResponse>;
    fn try_from(item: payment_methods::PaymentMethodCreateData) -> Result<Self, Self::Error> {
        match item {
            payment_methods::PaymentMethodCreateData::Card(card) => Ok(Self::Card(card)),
            payment_methods::PaymentMethodCreateData::ProxyCard(card) => Err(
                errors::api_error_response::ApiErrorResponse::UnprocessableEntity {
                    message: "Proxy Card for PaymentMethodCreateData".to_string(),
                }
                .into(),
            ),
        }
    }
}

impl From<PaymentMethodVaultingData> for PaymentMethodCustomVaultingData {
    fn from(item: PaymentMethodVaultingData) -> Self {
        match item {
            PaymentMethodVaultingData::Card(card_data) => Self::CardData(CardCustomData {
                card_number: Some(card_data.card_number),
                card_exp_month: Some(card_data.card_exp_month),
                card_exp_year: Some(card_data.card_exp_year),
                card_cvc: card_data.card_cvc,
            }),
            PaymentMethodVaultingData::NetworkToken(network_token_data) => {
                Self::NetworkTokenData(NetworkTokenCustomData {
                    network_token: Some(network_token_data.network_token),
                    network_token_exp_month: Some(network_token_data.network_token_exp_month),
                    network_token_exp_year: Some(network_token_data.network_token_exp_year),
                    cryptogram: network_token_data.cryptogram,
                })
            }
            PaymentMethodVaultingData::CardNumber(card_number_data) => {
                Self::CardData(CardCustomData {
                    card_number: Some(card_number_data),
                    card_exp_month: None,
                    card_exp_year: None,
                    card_cvc: None,
                })
            }
        }
    }
}
