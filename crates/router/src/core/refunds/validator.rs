use common_utils::ext_traits::StringExt;
use error_stack::{report, IntoReport, ResultExt};
use router_env::{instrument, tracing};
use time::PrimitiveDateTime;

use crate::{
    core::errors::{self, CustomResult, RouterResult},
    db::StorageInterface,
    logger,
    types::storage::{self, enums},
    utils::{self, OptionExt},
};

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
                Err(err
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while finding refund, database error"))
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

pub fn validate_refund_list(limit: Option<i64>) -> CustomResult<i64, errors::ApiErrorResponse> {
    match limit {
        Some(limit_val) => {
            if !(1..=100).contains(&limit_val) {
                Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "limit should be in between 1 and 100".to_string(),
                }
                .into())
            } else {
                Ok(limit_val)
            }
        }
        None => Ok(10),
    }
}

pub fn validate_for_valid_refunds(
    payment_attempt: &storage_models::payment_attempt::PaymentAttempt,
) -> RouterResult<()> {
    let connector: api_models::enums::Connector = payment_attempt
        .connector
        .clone()
        .get_required_value("connector")?
        .parse_enum("connector")
        .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)?;

    let payment_method = payment_attempt
        .payment_method
        .as_ref()
        .get_required_value("payment_method")?;

    match payment_method {
        storage_models::enums::PaymentMethod::PayLater
        | storage_models::enums::PaymentMethod::Wallet => {
            let payment_method_type = payment_attempt
                .payment_method_type
                .clone()
                .get_required_value("payment_method_type")?;

            utils::when(
                matches!(
                    (connector, payment_method_type),
                    (
                        api_models::enums::Connector::Braintree,
                        storage_models::enums::PaymentMethodType::Paypal,
                    ) | (
                        api_models::enums::Connector::Klarna,
                        storage_models::enums::PaymentMethodType::Klarna
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
