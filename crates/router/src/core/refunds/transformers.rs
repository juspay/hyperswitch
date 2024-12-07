use common_utils::types::{SplitPaymentsRequest, SplitRefund};
use error_stack::{report, Report};
use hyperswitch_domain_models::router_request_types;

use super::validator;
use crate::{core::errors, types::transformers::ForeignTryFrom};

impl ForeignTryFrom<(SplitRefund, SplitPaymentsRequest, Option<String>)>
    for router_request_types::SplitRefundsRequest
{
    type Error = Report<errors::ApiErrorResponse>;

    fn foreign_try_from(
        item: (SplitRefund, SplitPaymentsRequest, Option<String>),
    ) -> Result<Self, Self::Error> {
        let (refund_request, payment_charges, charge_id) = item;

        match refund_request {
            SplitRefund::StripeSplitRefund(stripe_refund) => match payment_charges {
                SplitPaymentsRequest::StripeSplitPayment(stripe_payment) => {
                    let charge_id = charge_id.ok_or_else(|| {
                        report!(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Missing `charge_id` in PaymentAttempt.")
                    })?;

                    let options = validator::validate_charge_refund(
                        &SplitRefund::StripeSplitRefund(stripe_refund.clone()),
                        &stripe_payment.charge_type,
                    )?;

                    Ok(Self::StripeSplitRefund(
                        router_request_types::StripeSplitRefund {
                            charge_id, // Use `charge_id` from `PaymentAttempt`
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
