//! Retries the `payment_attempt.payment_method_id` DB write when it fails after a
//! save-payment-method locker call already succeeded (#12904). Tracking data is IDs only - the
//! vault entry exists by the time this runs, so nothing sensitive needs to be persisted here.

use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use hyperswitch_domain_models::platform::{
    ProcessorKeyStore, ProcessorMerchantAccount, ProcessorMerchantId,
};
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

/// Terminal state after the retry budget is exhausted while the attempt is missing.
const ATTEMPT_NOT_FOUND: &str = "ATTEMPT_NOT_FOUND";

/// Outcome of one reconciliation pass.
enum Outcome {
    /// `payment_method_id` written.
    Updated,
    /// Another flow already recorded a `payment_method_id`; nothing left to reconcile.
    AlreadySet,
    /// The attempt was not visible on this reconciliation pass.
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

fn reconciliation_retry_delay(
    outcome: &errors::CustomResult<Outcome, errors::ApiErrorResponse>,
    retry_count: i32,
) -> Option<i32> {
    match outcome {
        Ok(Outcome::Updated | Outcome::AlreadySet) => None,
        Ok(Outcome::AttemptNotFound) | Err(_) => retry_delay_seconds(retry_count),
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
            &tracking_data.processor_merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to fetch merchant key store")?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(&tracking_data.processor_merchant_id, &key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to fetch merchant account")?;
    let storage_scheme =
        ProcessorMerchantAccount::from_untagged(merchant_account).get_storage_scheme();
    let processor_merchant_id =
        ProcessorMerchantId::from(tracking_data.processor_merchant_id.clone());
    let key_store = ProcessorKeyStore::from_untagged(key_store);

    let read_attempt = || {
        db.find_payment_attempt_by_payment_id_processor_merchant_id_attempt_id(
            &tracking_data.payment_id,
            &processor_merchant_id,
            &tracking_data.attempt_id,
            storage_scheme,
            &key_store,
        )
    };
    let payment_attempt = match read_attempt().await {
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

    if let Some(payment_method_id) = payment_attempt.payment_method_id.as_deref() {
        // Redis can contain the payment method even if appending the drainer entry failed.
        // Confirm the primary row using the value we read, preserving any method already there.
        return match db
            .repair_payment_method_id_in_postgres(
                &tracking_data.payment_id,
                &tracking_data.processor_merchant_id,
                &tracking_data.attempt_id,
                Some(payment_method_id),
                &tracking_data.updated_by,
            )
            .await
        {
            Ok(Some(true)) => Ok(Outcome::Updated),
            Ok(Some(false)) => Ok(Outcome::AlreadySet),
            Ok(None) => Err(
                error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("PostgreSQL did not confirm the recorded payment method"),
            ),
            Err(err) if err.current_context().is_db_not_found() => Ok(Outcome::AttemptNotFound),
            Err(err) => Err(err)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to reconcile payment_method_id in PostgreSQL"),
        };
    }

    let payment_attempt_update = PaymentAttemptUpdate::PaymentMethodDetailsUpdateIfUnset {
        payment_method_id: Some(tracking_data.payment_method_id.clone()),
        updated_by: tracking_data.updated_by.clone(),
    };

    match db
        .update_payment_attempt_with_attempt_id(
            payment_attempt,
            payment_attempt_update,
            storage_scheme,
            &key_store,
        )
        .await
    {
        Ok(_) => Ok(Outcome::Updated),
        Err(err)
            if err.current_context().is_db_not_found()
                || err.current_context().is_db_unique_violation() =>
        {
            // A miss can mean a concurrent write won. Recheck in the same pass so the
            // final retry also confirms a winner. If the read is stale, verify the writer.
            let latest = match read_attempt().await {
                Ok(attempt) => attempt.payment_method_id,
                Err(read_err) if read_err.current_context().is_db_not_found() => None,
                Err(read_err) => {
                    return Err(read_err)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Unable to re-read the payment attempt");
                }
            };
            match db
                .repair_payment_method_id_in_postgres(
                    &tracking_data.payment_id,
                    &tracking_data.processor_merchant_id,
                    &tracking_data.attempt_id,
                    latest.as_deref(),
                    &tracking_data.updated_by,
                )
                .await
            {
                Ok(Some(true)) => Ok(Outcome::Updated),
                Ok(Some(false)) => Ok(Outcome::AlreadySet),
                Ok(None) => Err(err)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Payment method is still unset after a conditional update miss",
                    ),
                Err(confirm_err) if confirm_err.current_context().is_db_not_found() => {
                    Ok(Outcome::AttemptNotFound)
                }
                Err(confirm_err) => Err(confirm_err)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to confirm the payment method after an update miss"),
            }
        }
        Err(err) => Err(err)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment attempt with payment_method_id"),
    }
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
        let outcome = reconcile(state, &tracking_data).await;
        if let Err(err) = &outcome {
            warn!(
                attempt_id = %tracking_data.attempt_id,
                retry_count,
                "save-payment-method attempt update failed: {err:?}"
            );
        }
        if let Some(schedule_time) =
            pt_utils::get_time_from_delta(reconciliation_retry_delay(&outcome, retry_count))
        {
            scheduler.retry_process(process, schedule_time).await?;
            return Ok(());
        }

        match outcome {
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
            Err(_) => {
                error!(
                    attempt_id = %tracking_data.attempt_id,
                    "save-payment-method attempt update retries exhausted; \
                     vault holds payment_method_id={} but payment_attempt.payment_method_id \
                     could not be confirmed - needs manual reconciliation",
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
            processor_merchant_id: id_type::MerchantId::try_from(std::borrow::Cow::Borrowed(
                "merchant_123",
            ))
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
    fn missing_attempt_is_retried_until_budget_is_exhausted() {
        let missing = Ok(Outcome::AttemptNotFound);
        assert_eq!(reconciliation_retry_delay(&missing, 0), Some(900));
        assert_eq!(reconciliation_retry_delay(&missing, 4), Some(300));
        assert_eq!(reconciliation_retry_delay(&missing, 5), None);
    }

    #[test]
    fn a_confirmed_payment_method_does_not_retry() {
        assert_eq!(
            reconciliation_retry_delay(&Ok(Outcome::AlreadySet), 0),
            None
        );
        assert_eq!(reconciliation_retry_delay(&Ok(Outcome::Updated), 0), None);
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
