use std::{collections::HashSet, time::Duration};

use api_models::payment_methods::{
    PaymentMethodDeletionType, PaymentMethodFingerprintCleanupFailure,
};
use common_utils::ext_traits::ValueExt;
use router_env::{instrument, tracing};
use scheduler::{consumer, workflows::ProcessTrackerWorkflow};

use crate::{
    core::{errors, payment_methods::fingerprint_cleanup::FingerprintCleanupTrackingData},
    logger::{error, info},
    routes::SessionState,
    types::storage,
};

const BATCH_SIZE: usize = 50;
const BATCH_DELAY: Duration = Duration::from_secs(5);
const FINGERPRINT_NOT_ELIGIBLE_REASON: &str =
    "payment method was not found or is not an Apple Pay or Google Pay payment method with no locker ID";
const PAYMENT_METHOD_NOT_FOUND_REASON: &str = "payment method was not found";
const BATCH_UPDATE_FAILED_REASON: &str = "database update failed for this batch";

pub struct PaymentMethodFingerprintCleanupWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PaymentMethodFingerprintCleanupWorkflow {
    #[instrument(skip_all, fields(flow = ?router_env::Flow::PaymentMethodFingerprintCleanup))]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let mut tracking_data: FingerprintCleanupTrackingData = process
            .tracking_data
            .clone()
            .parse_value("FingerprintCleanupTrackingData")
            .map_err(errors::ProcessTrackerError::from)?;

        while tracking_data.next_index < tracking_data.payment_method_ids.len() {
            let batch_start = tracking_data.next_index;
            let batch_end = (batch_start + BATCH_SIZE).min(tracking_data.payment_method_ids.len());
            let batch = tracking_data.payment_method_ids[batch_start..batch_end].to_vec();

            let update_result = match tracking_data.deletion_type {
                PaymentMethodDeletionType::LockerFingerprintId => {
                    state
                        .store
                        .clear_locker_fingerprint_ids_by_payment_method_ids(&batch)
                        .await
                }
                PaymentMethodDeletionType::LockerId => {
                    state
                        .store
                        .clear_locker_ids_by_payment_method_ids(&batch)
                        .await
                }
            };

            match update_result {
                Ok(updated_ids) => {
                    let updated_ids: HashSet<_> = updated_ids.into_iter().collect();
                    for payment_method_id in batch {
                        if updated_ids.contains(&payment_method_id) {
                            tracking_data.successful_ids += 1;
                        } else {
                            tracking_data
                                .failures
                                .push(PaymentMethodFingerprintCleanupFailure {
                                    payment_method_id,
                                    failure_reason: match tracking_data.deletion_type {
                                        PaymentMethodDeletionType::LockerFingerprintId => {
                                            FINGERPRINT_NOT_ELIGIBLE_REASON.to_string()
                                        }
                                        PaymentMethodDeletionType::LockerId => {
                                            PAYMENT_METHOD_NOT_FOUND_REASON.to_string()
                                        }
                                    },
                                });
                        }
                    }
                }
                Err(error) => {
                    error!(
                        job_id = %process.id,
                        batch_start,
                        batch_end,
                        error = ?error,
                        deletion_type = ?tracking_data.deletion_type,
                        "Failed to clear locker data for a payment method batch"
                    );
                    tracking_data
                        .failures
                        .extend(batch.into_iter().map(|payment_method_id| {
                            PaymentMethodFingerprintCleanupFailure {
                                payment_method_id,
                                failure_reason: BATCH_UPDATE_FAILED_REASON.to_string(),
                            }
                        }));
                }
            }

            tracking_data.next_index = batch_end;
            let tracking_value = serde_json::to_value(&tracking_data)
                .map_err(|_| errors::ProcessTrackerError::TypeConversionError)?;
            state
                .store
                .as_scheduler()
                .process_tracker_update_process_status_by_ids(
                    vec![process.id.clone()],
                    storage::ProcessTrackerUpdate::Update {
                        name: None,
                        retry_count: None,
                        schedule_time: None,
                        tracking_data: Some(tracking_value),
                        business_status: None,
                        status: None,
                        updated_at: Some(common_utils::date_time::now()),
                    },
                )
                .await
                .map_err(errors::ProcessTrackerError::from)?;

            info!(
                job_id = %process.id,
                batch_start,
                batch_end,
                total_ids = tracking_data.payment_method_ids.len(),
                successful_ids = tracking_data.successful_ids,
                failed_ids = tracking_data.failures.len(),
                deletion_type = ?tracking_data.deletion_type,
                "Processed payment method locker data cleanup batch"
            );

            if tracking_data.next_index < tracking_data.payment_method_ids.len() {
                tokio::time::sleep(BATCH_DELAY).await;
            }
        }

        state
            .store
            .as_scheduler()
            .finish_process_with_business_status(
                process,
                diesel_models::process_tracker::business_status::COMPLETED_BY_PT,
            )
            .await
            .map_err(errors::ProcessTrackerError::from)
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}
