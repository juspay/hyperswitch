//! Wallets the shopper is redirected to.

use super::{AciPayment, PaymentMethodAdapter};
use crate::{
    connectors::aci::transformers::{
        request::{PaymentBrand, PaymentDetails, WalletPMData},
        Error,
    },
    utils::{PhoneDetailsData as _, RouterData as _},
};

/// MB WAY, addressed by the shopper's billing phone number.
pub(in crate::connectors::aci::transformers::payment) struct MbWay;

impl PaymentMethodAdapter for MbWay {
    const CARD_ON_FILE: bool = false;

    fn instrument(self, payment: &AciPayment<'_>) -> Result<PaymentDetails, Error> {
        let phone = payment.item().router_data.get_billing_phone()?;
        Ok(PaymentDetails::Wallet(Box::new(WalletPMData {
            payment_brand: PaymentBrand::Mbway,
            account_id: Some(phone.get_number_with_hash_country_code()?),
        })))
    }
}

pub(in crate::connectors::aci::transformers::payment) struct AliPay;

impl PaymentMethodAdapter for AliPay {
    const CARD_ON_FILE: bool = false;

    fn instrument(self, _payment: &AciPayment<'_>) -> Result<PaymentDetails, Error> {
        Ok(PaymentDetails::Wallet(Box::new(WalletPMData {
            payment_brand: PaymentBrand::AliPay,
            account_id: None,
        })))
    }
}
