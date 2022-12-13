use error_stack::report;
use router_env::{tracing, tracing::instrument};
use time::PrimitiveDateTime;

use crate::{
    core::errors::{self, CustomResult, RouterResult},
    db::StorageInterface,
    logger,
    types::storage::{self, enums},
    utils,
};

pub(super) const REFUND_MAX_AGE: i64 = 365;
pub(super) const REFUND_MAX_ATTEMPTS: usize = 10;

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

//todo: max refund request count
#[instrument(skip_all)]
pub fn validate_refund_amount(
    payment_attempt_amount: i32, // &storage::PaymentAttempt,
    all_refunds: &[storage::Refund],
    refund_amount: i32,
) -> CustomResult<(), RefundValidationError> {
    let total_refunded_amount: i32 = all_refunds
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
        Err(report!(
            RefundValidationError::RefundAmountExceedsPaymentAmount
        )),
    )
}

#[instrument(skip_all)]
pub fn validate_payment_order_age(
    created_at: &PrimitiveDateTime,
) -> CustomResult<(), RefundValidationError> {
    let current_time = common_utils::date_time::now();

    utils::when(
        (current_time - *created_at).whole_days() > REFUND_MAX_AGE,
        Err(report!(RefundValidationError::OrderExpired)),
    )
}

#[instrument(skip_all)]
pub fn validate_maximum_refund_against_payment_attempt(
    all_refunds: &[storage::Refund],
) -> CustomResult<(), RefundValidationError> {
    // TODO: Make this configurable
    utils::when(
        all_refunds.len() > REFUND_MAX_ATTEMPTS,
        Err(report!(RefundValidationError::MaxRefundCountReached)),
    )
}

#[instrument(skip(db))]
pub async fn validate_uniqueness_of_refund_id_against_merchant_id(
    db: &dyn StorageInterface,
    payment_id: &str,
    merchant_id: &str,
    refund_id: &str,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<Option<storage::Refund>> {
    let refund = db
        .find_refund_by_merchant_id_refund_id(merchant_id, refund_id, storage_scheme)
        .await;
    logger::debug!(?refund);
    match refund {
        Err(err) => {
            if err.current_context().is_db_not_found() {
                // Empty vec should be returned by query in case of no results, this check exists just
                // to be on the safer side. Fixed this, now vector is not returned but should check the flow in detail later.
                Ok(None)
            } else {
                Err(err.change_context(errors::ApiErrorResponse::InternalServerError))
            }
        }

        Ok(refund) => {
            if refund.payment_id == payment_id {
                Ok(Some(refund))
            } else {
                Ok(None)
            }
        }
    }
}
