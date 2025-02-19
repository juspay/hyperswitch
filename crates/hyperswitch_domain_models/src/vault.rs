use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use api_models::payment_methods;

use crate::payment_method_data::NetworkTokenDetails;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PaymentMethodVaultingData {
    Card(payment_methods::CardDetail),
    NetworkToken(NetworkTokenDetails),
}

pub trait VaultingDataInterface {
    fn get_vaulting_data_key(&self) -> String;
}

impl VaultingDataInterface for PaymentMethodVaultingData {
    fn get_vaulting_data_key(&self) -> String {
        match &self {
            Self::Card(card) => card.card_number.to_string(),
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
