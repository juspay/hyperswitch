use error_stack::{report, IntoReport};
use router_env::{instrument, tracing};
use time::PrimitiveDateTime;

use crate::{
    core::errors::{self, CustomResult, RouterResult},
    types::storage::{self, enums},
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
    #[error("The refund amount exceeds the payment amount")]
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
    payment_attempt_amount: i64, // &storage::PaymentAttempt,
    all_refunds: &[storage::Refund],
    refund_amount: i64,
) -> CustomResult<(), RefundValidationError> {
    let total_refunded_amount: i64 = all_refunds
        .iter()
        .filter_map(|refund| {
            if refund.refund_status != enums::RefundStatus::Failure
                && refund.refund_status != enums::RefundStatus::TransactionFailure
            {
                Some(refund.refund_amount)
            } else {
                None
            }
        })
        .sum();

    utils::when(
        refund_amount > (payment_attempt_amount - total_refunded_amount),
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
    payment_attempt: &data_models::payments::payment_attempt::PaymentAttempt,
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
                    ) | (
                        api_models::enums::Connector::Klarna,
                        diesel_models::enums::PaymentMethodType::Klarna
                    )
                ),
                || {
                    Err(errors::ApiErrorResponse::RefundNotPossible {
                        connector: connector.to_string(),
                    })
                },
            )
            .into_report()
        }
        _ => Ok(()),
    }
}
