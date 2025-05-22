use api_models::payment_methods;
use serde::{Deserialize, Serialize};

use crate::payment_method_data;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PaymentMethodVaultingData {
    Card(payment_methods::CardDetail),
    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    NetworkToken(payment_method_data::NetworkTokenDetails),
}

impl PaymentMethodVaultingData {
    pub fn get_card(&self) -> Option<&payment_methods::CardDetail> {
        match self {
            Self::Card(card) => Some(card),
            #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
            Self::NetworkToken(_) => None,
        }
    }
    pub fn get_payment_methods_data(&self) -> payment_method_data::PaymentMethodsData {
        match self {
            Self::Card(card) => payment_method_data::PaymentMethodsData::Card(
                payment_method_data::CardDetailsPaymentMethod::from(card.clone()),
            ),
            #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
            Self::NetworkToken(network_token) => {
                payment_method_data::PaymentMethodsData::NetworkToken(
                    payment_method_data::NetworkTokenDetailsPaymentMethod::from(
                        network_token.clone(),
                    ),
                )
            }
        }
    }
}

pub trait VaultingDataInterface {
    fn get_vaulting_data_key(&self) -> String;
}

impl VaultingDataInterface for PaymentMethodVaultingData {
    fn get_vaulting_data_key(&self) -> String {
        match &self {
            Self::Card(card) => card.card_number.to_string(),
            #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
            Self::NetworkToken(network_token) => network_token.network_token.to_string(),
        }
    }
}

impl From<payment_methods::PaymentMethodCreateData> for PaymentMethodVaultingData {
    fn from(item: payment_methods::PaymentMethodCreateData) -> Self {
        match item {
            payment_methods::PaymentMethodCreateData::Card(card) => Self::Card(card),
        }
    }
}
