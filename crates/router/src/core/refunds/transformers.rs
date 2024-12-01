use common_utils::types::{SplitPaymentsRequest, SplitRefundRequest};
use error_stack::Report;
use hyperswitch_domain_models::router_request_types;

use super::validator;
use crate::{core::errors, types::transformers::ForeignTryFrom};

impl ForeignTryFrom<(SplitRefundRequest, SplitPaymentsRequest)>
    for router_request_types::SplitRefundsRequest
{
    type Error = Report<errors::ApiErrorResponse>;

    fn foreign_try_from(
        item: (SplitRefundRequest, SplitPaymentsRequest),
    ) -> Result<Self, Self::Error> {
        let (refund_request, payment_charges) = item;

        match refund_request {
            SplitRefundRequest::StripeSplitRefundRequest(stripe_refund) => match payment_charges {
                SplitPaymentsRequest::StripeSplitPayment(stripe_payment) => {
                    let options = validator::validate_charge_refund(
                        &SplitRefundRequest::StripeSplitRefundRequest(stripe_refund.clone()),
                        &stripe_payment.charge_type,
                    )?;

                    Ok(Self::StripeSplitRefund(
                        router_request_types::StripeSplitRefund {
                            charge_id: stripe_refund.charge_id,
                            transfer_account_id: stripe_payment.transfer_account_id,
                            charge_type: stripe_payment.charge_type,
                            options,
                        },
                    ))
                }
            },
        }
    }
}
