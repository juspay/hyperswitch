//! A raw network token.

use hyperswitch_domain_models::payment_method_data::NetworkTokenData;

use super::{AciPayment, PaymentMethodAdapter};
use crate::{
    connectors::aci::transformers::{
        request::{AciNetworkTokenData, AciTokenAccountType, PaymentBrand, PaymentDetails},
        Error,
    },
    utils::NetworkTokenData as _,
};

pub(in crate::connectors::aci::transformers::payment) struct NetworkToken<'a>(
    pub(in crate::connectors::aci::transformers::payment) &'a NetworkTokenData,
);

impl PaymentMethodAdapter for NetworkToken<'_> {
    const CARD_ON_FILE: bool = true;
    const LEGACY_RECURRING_TYPE: bool = false;

    fn instrument(self, _payment: &AciPayment<'_>) -> Result<PaymentDetails, Error> {
        let token = self.0;
        // A token that names no network is sent as VISA.
        let payment_brand = match token.card_network.as_ref() {
            None => PaymentBrand::Visa,
            network @ Some(_) => PaymentBrand::for_card_network(network)?,
        };
        Ok(PaymentDetails::AciNetworkToken(Box::new(
            AciNetworkTokenData {
                token_type: AciTokenAccountType::Network,
                token_number: token.get_network_token(),
                token_expiry_month: token.get_network_token_expiry_month(),
                token_expiry_year: token.get_expiry_year_4_digit(),
                token_cryptogram: Some(token.get_cryptogram().clone().unwrap_or_default()),
                eci: token.eci.clone(),
                payment_brand,
            },
        )))
    }
}
