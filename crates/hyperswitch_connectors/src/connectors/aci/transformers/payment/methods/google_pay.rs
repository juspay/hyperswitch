//! Google Pay: a decrypted token goes out as a network token; otherwise ACI decrypts the
//! encrypted payload itself.

use cards::NetworkToken;
use common_types::payments::{GPayPredecryptData, GpayTokenizationData};
use hyperswitch_domain_models::{
    payment_method_data::GooglePayWalletData, router_data::PaymentMethodToken,
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;

use super::{AciPayment, PaymentMethodAdapter};
use crate::connectors::aci::transformers::{
    request::{
        AciGooglePayEncryptedData, AciNetworkTokenData, AciTokenAccountType, PaymentBrand,
        PaymentDetails,
    },
    Error,
};

pub(in crate::connectors::aci::transformers::payment) struct GooglePay<'a>(
    pub(in crate::connectors::aci::transformers::payment) &'a GooglePayWalletData,
);

impl PaymentMethodAdapter for GooglePay<'_> {
    const CARD_ON_FILE: bool = true;

    fn instrument(self, payment: &AciPayment<'_>) -> Result<PaymentDetails, Error> {
        let wallet = self.0;
        match decrypted(
            wallet,
            payment.item().router_data.payment_method_token.as_ref(),
        ) {
            Some(decrypted) => {
                let payment_brand = PaymentBrand::for_wallet_network(&wallet.info.card_network)
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "google_pay.info.card_network".into(),
                    })?;
                Ok(PaymentDetails::AciNetworkToken(Box::new(
                    AciNetworkTokenData {
                        token_type: AciTokenAccountType::Network,
                        token_number: NetworkToken::from(
                            decrypted.application_primary_account_number.clone(),
                        ),
                        token_expiry_month: decrypted.card_exp_month.clone(),
                        token_expiry_year: decrypted.get_four_digit_expiry_year().map_err(
                            |_| errors::ConnectorError::MissingRequiredField {
                                field_name: "google_pay.card_exp_year".into(),
                            },
                        )?,
                        token_cryptogram: decrypted.cryptogram.clone(),
                        eci: decrypted.eci_indicator.clone(),
                        payment_brand,
                    },
                )))
            }
            None => {
                let encrypted = wallet
                    .tokenization_data
                    .get_encrypted_google_pay_token()
                    .map_err(|_| errors::ConnectorError::MissingRequiredField {
                        field_name: "google_pay.tokenization_data (encrypted)".into(),
                    })?;
                Ok(PaymentDetails::AciGooglePayEncrypted(Box::new(
                    AciGooglePayEncryptedData {
                        payment_token: Secret::new(encrypted),
                        source: "web".to_string(),
                        payment_brand: PaymentBrand::GooglePay,
                    },
                )))
            }
        }
    }
}

/// The decrypted payload: hyperswitch's own decryption first, then a payload the merchant
/// decrypted.
fn decrypted(
    wallet: &GooglePayWalletData,
    payment_method_token: Option<&PaymentMethodToken>,
) -> Option<GPayPredecryptData> {
    match (payment_method_token, &wallet.tokenization_data) {
        (Some(PaymentMethodToken::GooglePayDecrypt(decrypted)), _) => Some(*decrypted.clone()),
        (_, GpayTokenizationData::Decrypted(decrypted)) => Some(decrypted.clone()),
        (_, GpayTokenizationData::Encrypted(_)) => None,
    }
}
