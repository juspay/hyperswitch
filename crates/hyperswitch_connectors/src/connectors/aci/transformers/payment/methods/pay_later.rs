//! Buy now, pay later.

use hyperswitch_domain_models::payment_method_data::PayLaterData;

use super::{unsupported_payment_method, AciPayment, PaymentMethodAdapter};
use crate::connectors::aci::transformers::{
    request::{PaymentBrand, PaymentDetails, WalletPMData},
    Error,
};

pub(in crate::connectors::aci::transformers::payment) struct PayLater<'a>(
    pub(in crate::connectors::aci::transformers::payment) &'a PayLaterData,
);

impl PaymentMethodAdapter for PayLater<'_> {
    const CARD_ON_FILE: bool = false;

    /// Klarna rides the wallet carrier so its brand is actually serialized: a unit variant of the
    /// untagged, flattened [`PaymentDetails`] contributes no fields at all, which once left the
    /// request identifying no payment instrument.
    fn instrument(self, _payment: &AciPayment<'_>) -> Result<PaymentDetails, Error> {
        match self.0 {
            PayLaterData::KlarnaRedirect {} | PayLaterData::KlarnaSdk { .. } => {
                Ok(PaymentDetails::Wallet(Box::new(WalletPMData {
                    payment_brand: PaymentBrand::Klarna,
                    account_id: None,
                })))
            }
            _ => Err(unsupported_payment_method()),
        }
    }
}
