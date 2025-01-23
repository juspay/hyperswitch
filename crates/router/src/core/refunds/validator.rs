use error_stack::report;
use router_env::{instrument, tracing};
use time::PrimitiveDateTime;

use crate::{
    core::errors::{self, CustomResult, RouterResult},
    types::{
        self,
        api::enums as api_enums,
        storage::{self, enums},
    },
    utils::{self, OptionExt},
};

// Limit constraints for refunds list flow
pub const LOWER_LIMIT: i64 = 1;
pub const UPPER_LIMIT: i64 = 100;
pub const DEFAULT_LIMIT: i64 = 10;

#[derive(Debug, thiserror::Error)]
pub enum RefundValidationError {
    #[error("The payment attempt was not successful")]
    UnsuccessfulPaymentAttempt,
    #[error("The refund amount exceeds the amount captured")]
    RefundAmountExceedsPaymentAmount,
    #[error("The order has expired")]
    OrderExpired,
    #[error("The maximum refund count for this payment attempt")]
    MaxRefundCountReached,
    #[error("There is already another refund request for this payment attempt")]
    DuplicateRefund,
}

#[instrument(skip_all)]
pub fn validate_success_transaction(
    transaction: &storage::PaymentAttempt,
) -> CustomResult<(), RefundValidationError> {
    if transaction.status != enums::AttemptStatus::Charged {
        Err(report!(RefundValidationError::UnsuccessfulPaymentAttempt))?
    }

    Ok(())
}

#[instrument(skip_all)]
pub fn validate_refund_amount(
    amount_captured: i64,
    all_refunds: &[storage::Refund],
    refund_amount: i64,
) -> CustomResult<(), RefundValidationError> {
    let total_refunded_amount: i64 = all_refunds
        .iter()
        .filter_map(|refund| {
            if refund.refund_status != enums::RefundStatus::Failure
                && refund.refund_status != enums::RefundStatus::TransactionFailure
            {
                Some(refund.refund_amount.get_amount_as_i64())
            } else {
                None
            }
        })
        .sum();

    utils::when(
        refund_amount > (amount_captured - total_refunded_amount),
        || {
            Err(report!(
                RefundValidationError::RefundAmountExceedsPaymentAmount
            ))
        },
    )
}

#[instrument(skip_all)]
pub fn validate_payment_order_age(
    created_at: &PrimitiveDateTime,
    refund_max_age: i64,
) -> CustomResult<(), RefundValidationError> {
    let current_time = common_utils::date_time::now();

    utils::when(
        (current_time - *created_at).whole_days() > refund_max_age,
        || Err(report!(RefundValidationError::OrderExpired)),
    )
}

#[instrument(skip_all)]
pub fn validate_maximum_refund_against_payment_attempt(
    all_refunds: &[storage::Refund],
    refund_max_attempts: usize,
) -> CustomResult<(), RefundValidationError> {
    utils::when(all_refunds.len() > refund_max_attempts, || {
        Err(report!(RefundValidationError::MaxRefundCountReached))
    })
}

pub fn validate_refund_list(limit: Option<i64>) -> CustomResult<i64, errors::ApiErrorResponse> {
    match limit {
        Some(limit_val) => {
            if !(LOWER_LIMIT..=UPPER_LIMIT).contains(&limit_val) {
                Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "limit should be in between 1 and 100".to_string(),
                }
                .into())
            } else {
                Ok(limit_val)
            }
        }
        None => Ok(DEFAULT_LIMIT),
    }
}

pub fn validate_for_valid_refunds(
    payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    connector: api_models::enums::Connector,
) -> RouterResult<()> {
    let payment_method = payment_attempt
        .payment_method
        .as_ref()
        .get_required_value("payment_method")?;

    match payment_method {
        diesel_models::enums::PaymentMethod::PayLater
        | diesel_models::enums::PaymentMethod::Wallet => {
            let payment_method_type = payment_attempt
                .payment_method_type
                .get_required_value("payment_method_type")?;

            utils::when(
                matches!(
                    (connector, payment_method_type),
                    (
                        api_models::enums::Connector::Braintree,
                        diesel_models::enums::PaymentMethodType::Paypal,
                    )
                ),
                || {
                    Err(errors::ApiErrorResponse::RefundNotPossible {
                        connector: connector.to_string(),
                    }
                    .into())
                },
            )
        }
        _ => Ok(()),
    }
}

pub fn validate_stripe_charge_refund(
    charge_type_option: Option<api_enums::PaymentChargeType>,
    split_refund_request: &Option<common_types::refunds::SplitRefund>,
) -> RouterResult<types::ChargeRefundsOptions> {
    let charge_type = charge_type_option.ok_or_else(|| {
        report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Missing `charge_type` in PaymentAttempt.")
    })?;

    let refund_request = match split_refund_request {
        Some(common_types::refunds::SplitRefund::StripeSplitRefund(stripe_split_refund)) => {
            stripe_split_refund
        }
        _ => Err(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "stripe_split_refund",
        })?,
    };

    let options = match charge_type {
        api_enums::PaymentChargeType::Stripe(api_enums::StripeChargeType::Direct) => {
            types::ChargeRefundsOptions::Direct(types::DirectChargeRefund {
                revert_platform_fee: refund_request
                    .revert_platform_fee
                    .get_required_value("revert_platform_fee")?,
            })
        }
        api_enums::PaymentChargeType::Stripe(api_enums::StripeChargeType::Destination) => {
            types::ChargeRefundsOptions::Destination(types::DestinationChargeRefund {
                revert_platform_fee: refund_request
                    .revert_platform_fee
                    .get_required_value("revert_platform_fee")?,
                revert_transfer: refund_request
                    .revert_transfer
                    .get_required_value("revert_transfer")?,
            })
        }
    };

    Ok(options)
}

pub fn validate_adyen_charge_refund(
    adyen_split_payment_response: &common_types::domain::AdyenSplitData,
    adyen_split_refund_request: &common_types::domain::AdyenSplitData,
) -> RouterResult<()> {
    if adyen_split_refund_request.store != adyen_split_payment_response.store {
        return Err(report!(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "split_payments.adyen_split_payment.store",
        }));
    };

    for refund_split_item in adyen_split_refund_request.split_items.iter() {
        let refund_split_reference = refund_split_item.reference.clone();
        let matching_payment_split_item = adyen_split_payment_response
            .split_items
            .iter()
            .find(|payment_split_item| refund_split_reference == payment_split_item.reference);

        if let Some(payment_split_item) = matching_payment_split_item {
            if let Some((refund_amount, payment_amount)) =
                refund_split_item.amount.zip(payment_split_item.amount)
            {
                if refund_amount > payment_amount {
                    return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                        message: format!(
                            "Invalid refund amount for split item, reference: {}",
                            refund_split_reference
                        ),
                    }));
                }
            }

            if let Some((refund_account, payment_account)) = refund_split_item
                .account
                .as_ref()
                .zip(payment_split_item.account.as_ref())
            {
                if !refund_account.eq(payment_account) {
                    return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                        message: format!(
                            "Invalid refund account for split item, reference: {}",
                            refund_split_reference
                        ),
                    }));
                }
            }

            if refund_split_item.split_type != payment_split_item.split_type {
                return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: format!(
                        "Invalid refund split_type for split item, reference: {}",
                        refund_split_reference
                    ),
                }));
            }
        } else {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: format!(
                    "No matching payment split item found for reference: {}",
                    refund_split_reference
                ),
            }));
        }
    }
    Ok(())
}
