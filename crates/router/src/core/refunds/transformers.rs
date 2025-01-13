use error_stack::{report, Report};
use hyperswitch_domain_models::router_request_types;

use super::validator;
use crate::core::errors;

pub struct SplitRefundInput {
    pub refund_request: common_types::refunds::SplitRefund,
    pub payment_charges: common_types::payments::SplitPaymentsRequest,
    pub charge_id: Option<String>,
}

impl TryFrom<SplitRefundInput> for router_request_types::SplitRefundsRequest {
    type Error = Report<errors::ApiErrorResponse>;

    fn try_from(value: SplitRefundInput) -> Result<Self, Self::Error> {
        let SplitRefundInput {
            refund_request,
            payment_charges,
            charge_id,
        } = value;

        match refund_request {
            common_types::refunds::SplitRefund::StripeSplitRefund(stripe_refund) => {
                match payment_charges {
                    common_types::payments::SplitPaymentsRequest::StripeSplitPayment(
                        stripe_payment,
                    ) => {
                        let charge_id = charge_id.ok_or_else(|| {
                            report!(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable("Missing `charge_id` in PaymentAttempt.")
                        })?;

                        let options = validator::validate_charge_refund(
                            &common_types::refunds::SplitRefund::StripeSplitRefund(
                                stripe_refund.clone(),
                            ),
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
                    common_types::payments::SplitPaymentsRequest::AdyenSplitPayment(_) => todo!(), // todooooo
                }
            }
        }
    }
}
