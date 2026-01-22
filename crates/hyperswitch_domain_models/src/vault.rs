use api_models::payment_methods;
#[cfg(feature = "v2")]
use common_utils::{crypto::Encryptable, errors::CustomResult, ext_traits::OptionExt};
#[cfg(feature = "v2")]
use error_stack::ResultExt;
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

    #[cfg(feature = "v2")]
    pub fn populated_payment_methods_data_and_get_card_details(
        &self,
        payment_methods_data_optional: Option<&Encryptable<payment_methods::PaymentMethodsData>>,
    ) -> CustomResult<
        Option<payment_methods::CardDetail>,
        errors::api_error_response::ApiErrorResponse,
    > {
        match self {
            Self::Card(card_details) => Ok(Some(card_details.clone())),
            Self::NetworkToken(_) => Ok(None),
            Self::CardNumber(card_number) => {
                let payment_methods_data = payment_methods_data_optional
                    .get_required_value("payment methods data")
                    .change_context(
                            errors::api_error_response::ApiErrorResponse::InternalServerError,
                        )
                    .attach_printable("failed to get payment methods data for payment method vaulting data type card number")?;
                let stored_card_metadata_optional =
                    payment_methods_data.clone().into_inner().get_card_details();

                if let Some(stored_card_metadata) = stored_card_metadata_optional {
                    let card_with_details = payment_method_data::CardNumberWithStoredDetails::new(
                        card_number.clone(),
                        stored_card_metadata.into(),
                    );

                    let card_detail = payment_methods::CardDetail::try_from(card_with_details)
                        .change_context(
                            errors::api_error_response::ApiErrorResponse::InternalServerError,
                        )
                        .attach_printable("Failed to create card details for payment method vaulting data type card number ")?;

                    Ok(Some(card_detail))
                } else {
                    Ok(None)
                }
            }
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
