//! Samsung Pay: always app-based, always decrypted by ACI.

use hyperswitch_domain_models::payment_method_data::SamsungPayWalletData;
use hyperswitch_interfaces::errors;

use super::{AciPayment, PaymentMethodAdapter};
use crate::connectors::aci::transformers::{
    request::{AciSamsungPayData, PaymentBrand, PaymentDetails},
    Error,
};

pub(in crate::connectors::aci::transformers::payment) struct SamsungPay<'a>(
    pub(in crate::connectors::aci::transformers::payment) &'a SamsungPayWalletData,
);

impl PaymentMethodAdapter for SamsungPay<'_> {
    const CARD_ON_FILE: bool = true;

    fn instrument(self, _payment: &AciPayment<'_>) -> Result<PaymentDetails, Error> {
        let credential = &self.0.payment_credential;
        // Reject an unknown brand before we POST. OPPWA decrypts the token and derives the scheme
        // itself, so `samsungPay.cardBrand` is not sent.
        if credential.card_brand == common_enums::SamsungPayCardBrand::Unknown {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "samsung_pay.card_brand".into(),
            })?
        }
        Ok(PaymentDetails::AciSamsungPay(Box::new(AciSamsungPayData {
            payment_token: credential.token_data.data.clone(),
            source: "app".to_string(),
            payment_brand: PaymentBrand::SamsungPay,
        })))
    }
}
