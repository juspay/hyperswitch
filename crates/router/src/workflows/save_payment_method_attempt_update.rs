//! Retries the `payment_attempt.payment_method_id` DB write when it fails after a
//! save-payment-method locker call already succeeded (#12904). Tracking data is IDs only - the
//! vault entry exists by the time this runs, so nothing sensitive needs to be persisted here.

use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use scheduler::{
    consumer::{self, types::process_data},
    utils as pt_utils,
    workflows::ProcessTrackerWorkflow,
};

use crate::{
    errors,
    logger::{error, warn},
    routes::{metrics, SessionState},
    types::storage::{
        self, payment_attempt::SavePaymentMethodAttemptUpdateTrackingData, PaymentAttemptUpdate,
    },
};

pub struct SavePaymentMethodAttemptUpdateWorkflow;

/// Terminal state for a task whose attempt row no longer resolves. Attempt rows are never
/// deleted, so this means the tracking data is bad, not that the DB is flaky - do not retry.
const ATTEMPT_NOT_FOUND: &str = "ATTEMPT_NOT_FOUND";

/// Outcome of one reconciliation pass.
enum Outcome {
    /// `payment_method_id` written.
    Updated,
    /// Another flow already recorded a `payment_method_id`; nothing left to reconcile.
    AlreadySet,
    /// The attempt referenced by the tracking data does not exist.
    AttemptNotFound,
}

/// Delay in seconds before the next run, or `None` once the retry budget is spent.
/// Mirrors `PaymentMethodStatusUpdateWorkflow`: `start_after` for the first retry, then the
/// `frequencies` schedule.
fn retry_delay_seconds(retry_count: i32) -> Option<i32> {
    let mapping = process_data::PaymentMethodsPTMapping::default();
    if retry_count == 0 {
        Some(mapping.default_mapping.start_after)
    } else {
        pt_utils::get_delay(retry_count + 1, &mapping.default_mapping.frequencies)
    }
}

/// Every DB read here is part of the retried path: a flaky DB is the reason this task exists,
/// so a failing key-store or attempt lookup must schedule a retry, not abort the workflow.
async fn reconcile(
    state: &SessionState,
    tracking_data: &SavePaymentMethodAttemptUpdateTrackingData,
) -> errors::CustomResult<Outcome, errors::ApiErrorResponse> {
    let db = &*state.store;

    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            &tracking_data.merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to fetch merchant key store")?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(&tracking_data.merchant_id, &key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to fetch merchant account")?;

    let payment_attempt = match db
        .find_payment_attempt_by_payment_id_processor_merchant_id_attempt_id(
            &tracking_data.payment_id,
            &tracking_data.merchant_id,
            &tracking_data.attempt_id,
            merchant_account.storage_scheme,
            &key_store,
        )
        .await
    {
        Ok(attempt) => attempt,
        Err(err) if err.current_context().is_db_not_found() => {
            return Ok(Outcome::AttemptNotFound);
        }
        Err(err) => {
            return Err(err)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to fetch payment attempt");
        }
    };

    // If a later flow (e.g. a manual retry from the merchant) already recorded a
    // payment_method_id, this task's write would be stale.
    if payment_attempt.payment_method_id.is_some() {
        return Ok(Outcome::AlreadySet);
    }

    let payment_attempt_update = PaymentAttemptUpdate::PaymentMethodDetailsUpdate {
        payment_method_id: Some(tracking_data.payment_method_id.clone()),
        updated_by: tracking_data.updated_by.clone(),
    };

    db.update_payment_attempt_with_attempt_id(
        payment_attempt,
        payment_attempt_update,
        merchant_account.storage_scheme,
        &key_store,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Unable to update payment attempt with payment_method_id")?;

    Ok(Outcome::Updated)
}

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for SavePaymentMethodAttemptUpdateWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let scheduler = state.store.as_scheduler();
        let tracking_data: SavePaymentMethodAttemptUpdateTrackingData = process
            .tracking_data
            .clone()
            .parse_value("SavePaymentMethodAttemptUpdateTrackingData")?;

        let retry_count = process.retry_count;

        match reconcile(state, &tracking_data).await {
            Ok(Outcome::Updated) => {
                scheduler
                    .finish_process_with_business_status(
                        process,
                        storage::business_status::COMPLETED_BY_PT,
                    )
                    .await?;
            }
            Ok(Outcome::AlreadySet) => {
                scheduler
                    .finish_process_with_business_status(process, "PROCESS_ALREADY_COMPLETED")
                    .await?;
            }
            Ok(Outcome::AttemptNotFound) => {
                error!(
                    attempt_id = %tracking_data.attempt_id,
                    payment_id = %tracking_data.payment_id.get_string_repr(),
                    "save-payment-method attempt update: payment attempt not found; \
                     vault holds payment_method_id={} that no attempt references",
                    tracking_data.payment_method_id,
                );
                metrics::SAVE_PAYMENT_METHOD_ATTEMPT_UPDATE_ATTEMPT_NOT_FOUND.add(1, &[]);
                scheduler
                    .finish_process_with_business_status(process, ATTEMPT_NOT_FOUND)
                    .await?;
            }
            Err(err) => {
                warn!(
                    attempt_id = %tracking_data.attempt_id,
                    retry_count,
                    "save-payment-method attempt update failed: {err:?}"
                );
                match pt_utils::get_time_from_delta(retry_delay_seconds(retry_count)) {
                    Some(schedule_time) => {
                        scheduler.retry_process(process, schedule_time).await?;
                    }
                    None => {
                        error!(
                            attempt_id = %tracking_data.attempt_id,
                            "save-payment-method attempt update retries exhausted; \
                             vault holds payment_method_id={} but payment_attempt.payment_method_id \
                             was never persisted - needs manual reconciliation",
                            tracking_data.payment_method_id,
                        );
                        metrics::SAVE_PAYMENT_METHOD_ATTEMPT_UPDATE_RETRIES_EXCEEDED.add(1, &[]);
                        scheduler
                            .finish_process_with_business_status(
                                process,
                                storage::business_status::RETRIES_EXCEEDED,
                            )
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        // Anything that escapes `execute_workflow` (e.g. unparseable tracking data) would
        // otherwise leave the row in ProcessStarted forever; finish it as GLOBAL_ERROR so it is
        // queryable.
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}

#[cfg(test)]
mod tests {
    use common_utils::{ext_traits::Encode, id_type};

    use super::*;

    #[test]
    fn test_tracking_data_round_trips_through_json() {
        let tracking_data = SavePaymentMethodAttemptUpdateTrackingData {
            attempt_id: "attempt_123".to_string(),
            payment_id: id_type::PaymentId::wrap("payment_123".to_string())
                .expect("valid payment_id"),
            merchant_id: id_type::MerchantId::try_from(std::borrow::Cow::Borrowed("merchant_123"))
                .expect("valid merchant_id"),
            payment_method_id: "pm_123".to_string(),
            updated_by: "psql".to_string(),
        };

        let encoded = tracking_data
            .encode_to_value()
            .expect("tracking data must serialize");
        let decoded: SavePaymentMethodAttemptUpdateTrackingData =
            serde_json::from_value(encoded).expect("tracking data must deserialize");

        assert_eq!(decoded.attempt_id, tracking_data.attempt_id);
        assert_eq!(decoded.payment_method_id, tracking_data.payment_method_id);
        assert_eq!(decoded.updated_by, tracking_data.updated_by);
    }

    #[test]
    fn test_retry_schedule_first_retry_after_start_after_then_frequencies_then_exhausted() {
        // Default mapping: start_after 900s, then (300s x 5).
        assert_eq!(retry_delay_seconds(0), Some(900));
        for retry_count in 1..=4 {
            assert_eq!(
                retry_delay_seconds(retry_count),
                Some(300),
                "retry {retry_count}"
            );
        }
        assert_eq!(retry_delay_seconds(5), None);
    }
}
