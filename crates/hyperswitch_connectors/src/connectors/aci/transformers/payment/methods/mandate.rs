//! A charge against an ACI registration.

use super::{AciPayment, PaymentMethodAdapter};
use crate::connectors::aci::transformers::{request::PaymentDetails, Error};

/// The stored credential is addressed through the request URL
/// (`/v1/registrations/{id}/payments`), so the body carries no instrument at all.
pub(in crate::connectors::aci::transformers::payment) struct Mandate;

impl PaymentMethodAdapter for Mandate {
    const CARD_ON_FILE: bool = true;

    fn instrument(self, _payment: &AciPayment<'_>) -> Result<PaymentDetails, Error> {
        Ok(PaymentDetails::Mandate)
    }
}
