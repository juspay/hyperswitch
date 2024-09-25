use common_utils::{ext_traits::ValueExt, pii, types::ChargeRefunds};
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::router_request_types;
use masking::PeekInterface;

use super::validator;
use crate::{core::errors, types::transformers::ForeignTryFrom};

impl ForeignTryFrom<(ChargeRefunds, pii::SecretSerdeValue)>
    for router_request_types::ChargeRefunds
{
    type Error = Report<errors::ApiErrorResponse>;
    fn foreign_try_from(item: (ChargeRefunds, pii::SecretSerdeValue)) -> Result<Self, Self::Error> {
        let (refund_charges, charges) = item;
        let payment_charges: router_request_types::PaymentCharges = charges
            .peek()
            .clone()
            .parse_value("PaymentCharges")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse charges into PaymentCharges")?;

        Ok(Self {
            charge_id: refund_charges.charge_id.clone(),
            charge_type: payment_charges.charge_type.clone(),
            transfer_account_id: payment_charges.transfer_account_id,
            options: validator::validate_charge_refund(
                &refund_charges,
                &payment_charges.charge_type,
            )?,
        })
    }
}
