//! Apple Pay: a decrypted token goes out as a network token; otherwise ACI decrypts the
//! encrypted payload itself.

use cards::NetworkToken;
use common_types::payments::{ApplePayPaymentData, ApplePayPredecryptData};
use hyperswitch_domain_models::{
    payment_method_data::ApplePayWalletData, router_data::PaymentMethodToken,
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;

use super::{AciPayment, PaymentMethodAdapter};
use crate::connectors::aci::transformers::{
    request::{
        AciApplePayEncryptedData, AciNetworkTokenData, AciTokenAccountType, PaymentBrand,
        PaymentDetails,
    },
    Error,
};

pub(in crate::connectors::aci::transformers::payment) struct ApplePay<'a>(
    pub(in crate::connectors::aci::transformers::payment) &'a ApplePayWalletData,
);

impl PaymentMethodAdapter for ApplePay<'_> {
    const CARD_ON_FILE: bool = true;

    fn instrument(self, payment: &AciPayment<'_>) -> Result<PaymentDetails, Error> {
        let wallet = self.0;
        match decrypted(
            wallet,
            payment.item().router_data.payment_method_token.as_ref(),
        ) {
            Some(decrypted) => {
                let payment_brand = PaymentBrand::for_wallet_network(
                    &wallet.payment_method.network,
                )
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "apple_pay.payment_method.network".into(),
                })?;
                Ok(PaymentDetails::AciNetworkToken(Box::new(
                    AciNetworkTokenData {
                        token_type: AciTokenAccountType::Network,
                        token_number: NetworkToken::from(
                            decrypted.application_primary_account_number.clone(),
                        ),
                        token_expiry_month: decrypted.application_expiration_month.clone(),
                        token_expiry_year: decrypted.get_four_digit_expiry_year(),
                        token_cryptogram: Some(
                            decrypted.payment_data.online_payment_cryptogram.clone(),
                        ),
                        eci: decrypted.payment_data.eci_indicator.clone(),
                        payment_brand,
                    },
                )))
            }
            None => {
                let encrypted = wallet
                    .payment_data
                    .get_encrypted_apple_pay_payment_data_optional()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "apple_pay.payment_data (encrypted)".into(),
                    })?;
                Ok(PaymentDetails::AciApplePayEncrypted(Box::new(
                    AciApplePayEncryptedData {
                        payment_token: Secret::new(encrypted.clone()),
                        source: "web".to_string(),
                        payment_brand: PaymentBrand::ApplePay,
                    },
                )))
            }
        }
    }
}

/// The decrypted payload: hyperswitch's own decryption first, then a payload the merchant
/// decrypted.
fn decrypted(
    wallet: &ApplePayWalletData,
    payment_method_token: Option<&PaymentMethodToken>,
) -> Option<ApplePayPredecryptData> {
    match (payment_method_token, &wallet.payment_data) {
        (Some(PaymentMethodToken::ApplePayDecrypt(decrypted)), _) => Some(*decrypted.clone()),
        (_, ApplePayPaymentData::Decrypted(decrypted)) => Some(decrypted.clone()),
        (_, ApplePayPaymentData::Encrypted(_)) => None,
    }
}
