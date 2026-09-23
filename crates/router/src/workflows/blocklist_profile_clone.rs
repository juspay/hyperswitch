use common_utils::{ext_traits::ValueExt, id_type};
use error_stack::ResultExt;
use router_env::{instrument, tracing};
use scheduler::{
    consumer::types::process_data, utils as pt_utils, workflows::ProcessTrackerWorkflow,
};

use crate::{
    core::{
        blocklist::clone,
        errors::{self, RouterResult},
    },
    logger::{error, info},
    routes::SessionState,
    types::storage,
};

pub struct BlocklistProfileCloneWorkflow;

/// Copies the source profile onto one target profile a page at a time.
async fn run_clone_job(
    state: &SessionState,
    process_id: &str,
    tracking_data: &mut storage::BlocklistProfileCloneTrackingData,
    merchant_id_obj: &id_type::MerchantId,
    target_profile_id: &id_type::ProfileId,
) -> RouterResult<i32> {
    let db = &*state.store;
    let job_id = tracking_data.job_id.clone();
    let merchant_id_str = merchant_id_obj.get_string_repr();

    loop {
        let page = match tracking_data.draining_legacy_rows {
            false => {
                db.list_blocklist_entries_after_fingerprint_by_processor_merchant_id_profile_id(
                    merchant_id_obj,
                    &tracking_data.source_profile_id,
                    tracking_data.last_fingerprint_id.clone(),
                    tracking_data.snapshot_at,
                    clone::CLONE_PAGE_SIZE,
                )
                .await
            }
            true => {
                db.list_blocklist_entries_after_fingerprint_by_legacy_merchant_id_profile_id(
                    merchant_id_obj,
                    &tracking_data.source_profile_id,
                    tracking_data.last_fingerprint_id.clone(),
                    tracking_data.snapshot_at,
                    clone::CLONE_PAGE_SIZE,
                )
                .await
            }
        }
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("Failed to read a source page for clone job {job_id}"))?;

        match page.last().map(|entry| entry.fingerprint_id.clone()) {
            // End of a pass: switch to the legacy rows, or stop if that pass is already done.
            None => match tracking_data.draining_legacy_rows {
                true => break,
                false => {
                    tracking_data.draining_legacy_rows = true;
                    tracking_data.last_fingerprint_id = String::new();
                    persist_cursor(state, process_id, tracking_data).await?;
                }
            },
            Some(cursor) => {
                let page_len = page.len();
                let now = common_utils::date_time::now();
                let entries = page
                    .into_iter()
                    .map(|entry| clone::to_target_entry(entry, target_profile_id, now))
                    .collect::<Vec<_>>();

                let inserted = db
                    .bulk_insert_blocklist_entries(entries)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable_lazy(|| {
                        format!("Failed to insert cloned entries for job {job_id}")
                    })?;

                let page_rows = i32::try_from(page_len)
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;
                tracking_data.processed_rows = tracking_data
                    .processed_rows
                    .checked_add(page_rows)
                    .ok_or(errors::ApiErrorResponse::InternalServerError)?;

                tracking_data.last_fingerprint_id = cursor;
                persist_cursor(state, process_id, tracking_data).await?;

                db.update_profile_clone_job_target(
                    &job_id,
                    merchant_id_str,
                    target_profile_id,
                    storage::BlocklistProfileCloneTargetUpdate {
                        status: common_enums::BatchBlocklistJobStatus::Processing,
                        processed_rows: tracking_data.processed_rows,
                        error_message: None,
                    },
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable_lazy(|| {
                    format!("Failed to update target progress for clone job {job_id}")
                })?;

                info!(
                    job_id = %job_id,
                    target_profile_id = ?target_profile_id,
                    page_len,
                    inserted,
                    processed_rows = tracking_data.processed_rows,
                    legacy_pass = tracking_data.draining_legacy_rows,
                    "Copied a page of blocklist entries onto the target profile"
                );
            }
        }
    }

    Ok(tracking_data.processed_rows)
}

/// Rewrites the process's tracking data so a retry picks up where this run left off.
async fn persist_cursor(
    state: &SessionState,
    process_id: &str,
    tracking_data: &storage::BlocklistProfileCloneTrackingData,
) -> RouterResult<()> {
    let td_value = serde_json::to_value(tracking_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialise updated tracking_data")?;

    state
        .store
        .as_scheduler()
        .process_tracker_update_process_status_by_ids(
            vec![process_id.to_owned()],
            storage::ProcessTrackerUpdate::Update {
                name: None,
                retry_count: None,
                schedule_time: None,
                tracking_data: Some(td_value),
                business_status: None,
                status: None,
                updated_at: Some(common_utils::date_time::now()),
            },
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to advance the cursor for a blocklist clone job")?;

    Ok(())
}

/// Returns the delay for another copy attempt, or None once copy retries are exhausted.
fn copy_retry_delay(retry_count: i32) -> Option<i32> {
    let mapping = process_data::RetryMapping::default();
    if retry_count == 0 {
        Some(mapping.start_after)
    } else {
        pt_utils::get_delay(retry_count.saturating_add(1), &mapping.frequencies)
    }
}

/// Copies the current target and records it as completed in the job's metadata.
async fn settle_current_target(
    state: &SessionState,
    process_id: &str,
    tracking_data: &mut storage::BlocklistProfileCloneTrackingData,
    merchant_id: &id_type::MerchantId,
    target_profile_id: &id_type::ProfileId,
) -> RouterResult<()> {
    let db = &*state.store;

    run_clone_job(
        state,
        process_id,
        tracking_data,
        merchant_id,
        target_profile_id,
    )
    .await?;

    db.update_profile_clone_job_target(
        &tracking_data.job_id,
        merchant_id.get_string_repr(),
        target_profile_id,
        storage::BlocklistProfileCloneTargetUpdate {
            status: common_enums::BatchBlocklistJobStatus::Completed,
            processed_rows: tracking_data.processed_rows,
            error_message: None,
        },
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to record a clone target as completed")?;

    info!(
        job_id = %tracking_data.job_id,
        target_profile_id = ?target_profile_id,
        "Blocklist profile clone target completed"
    );
    Ok(())
}

/// Points the tracking data at the next target with a fresh cursor, returning whether one remains.
fn move_to_next_target(tracking_data: &mut storage::BlocklistProfileCloneTrackingData) -> bool {
    tracking_data.current_target = tracking_data.current_target.saturating_add(1);
    tracking_data.draining_legacy_rows = false;
    tracking_data.last_fingerprint_id = String::new();
    tracking_data.processed_rows = 0;
    tracking_data.current_target < tracking_data.target_profile_ids.len()
}

/// Requeues the tracker for the next target with a fresh copy-retry budget.
async fn requeue_for_next_target(
    state: &SessionState,
    process_id: &str,
    tracking_data: &storage::BlocklistProfileCloneTrackingData,
) -> RouterResult<()> {
    let td_value = serde_json::to_value(tracking_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialise updated tracking_data")?;
    let now = common_utils::date_time::now();

    state
        .store
        .as_scheduler()
        .process_tracker_update_process_status_by_ids(
            vec![process_id.to_owned()],
            storage::ProcessTrackerUpdate::Update {
                name: None,
                retry_count: Some(0),
                schedule_time: Some(now),
                tracking_data: Some(td_value),
                business_status: None,
                status: Some(common_enums::ProcessTrackerStatus::New),
                updated_at: Some(now),
            },
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to requeue the blocklist clone job for its next target")?;

    Ok(())
}

const CLONE_FAILED_MESSAGE: &str = "Clone failed after repeated attempts. Please retry.";

/// Records the current target as failed once its copy retries are spent and requeues the tracker
/// for the next target. Returns whether it was requeued; `false` means no target remains.
async fn fail_current_target(
    state: &SessionState,
    process: &storage::ProcessTracker,
) -> RouterResult<bool> {
    let mut tracking_data: storage::BlocklistProfileCloneTrackingData = process
        .tracking_data
        .clone()
        .parse_value("BlocklistProfileCloneTrackingData")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let merchant_id = tracking_data
        .processor_merchant_id
        .clone()
        .unwrap_or_else(|| tracking_data.merchant_id.clone());
    let target_profile_id = tracking_data
        .target_profile_ids
        .get(tracking_data.current_target)
        .cloned()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Blocklist clone tracker points past its last target")?;

    state
        .store
        .update_profile_clone_job_target(
            &tracking_data.job_id,
            merchant_id.get_string_repr(),
            &target_profile_id,
            storage::BlocklistProfileCloneTargetUpdate {
                status: common_enums::BatchBlocklistJobStatus::Failed,
                processed_rows: tracking_data.processed_rows,
                error_message: Some(CLONE_FAILED_MESSAGE.to_string()),
            },
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to record a clone target as failed")?;

    match move_to_next_target(&mut tracking_data) {
        true => requeue_for_next_target(state, &process.id, &tracking_data)
            .await
            .map(|()| true),
        false => Ok(false),
    }
}

/// Marks the whole job failed without reading its metadata, so a job whose metadata is missing or
/// malformed still settles. Best effort: the caller finishes the tracker either way.
async fn fail_job(state: &SessionState, process: &storage::ProcessTracker) {
    let parsed: Result<storage::BlocklistProfileCloneTrackingData, _> = process
        .tracking_data
        .clone()
        .parse_value("BlocklistProfileCloneTrackingData");

    match parsed {
        Ok(tracking_data) => {
            let merchant_id = tracking_data
                .processor_merchant_id
                .clone()
                .unwrap_or_else(|| tracking_data.merchant_id.clone());
            state
                .store
                .update_batch_blocklist_job_by_id_merchant_id(
                    &tracking_data.job_id,
                    merchant_id.get_string_repr(),
                    storage::BatchBlocklistJobUpdate {
                        status: Some(common_enums::BatchBlocklistJobStatus::Failed),
                        succeeded_rows: None,
                        failed_rows: None,
                        total_rows: None,
                        file_key: None,
                        error_message: Some(CLONE_FAILED_MESSAGE.to_string()),
                        expires_at: None,
                        metadata: None,
                        updated_at: common_utils::date_time::now(),
                    },
                )
                .await
                .inspect_err(|error| {
                    error!(
                        ?error,
                        job_id = %tracking_data.job_id,
                        "Failed to mark the blocklist clone job as failed"
                    )
                })
                .ok();
        }
        Err(error) => error!(
            ?error,
            process_id = %process.id,
            "Unreadable blocklist clone tracking data; the job cannot be marked failed"
        ),
    }
}

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for BlocklistProfileCloneWorkflow {
    /// Settles one target per run. Between targets the tracker is requeued with its retry count
    /// reset, so every target gets its own copy-retry budget; after the last one it finishes.
    #[instrument(skip_all, fields(flow = ?router_env::Flow::CloneBlocklistEntries))]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let mut tracking_data: storage::BlocklistProfileCloneTrackingData = process
            .tracking_data
            .clone()
            .parse_value("BlocklistProfileCloneTrackingData")
            .map_err(errors::ProcessTrackerError::from)?;
        let merchant_id = tracking_data
            .processor_merchant_id
            .clone()
            .unwrap_or_else(|| tracking_data.merchant_id.clone());
        let target_profile_id = tracking_data
            .target_profile_ids
            .get(tracking_data.current_target)
            .cloned()
            .ok_or(errors::ProcessTrackerError::UnexpectedFlow)?;

        settle_current_target(
            state,
            &process.id,
            &mut tracking_data,
            &merchant_id,
            &target_profile_id,
        )
        .await
        .map_err(errors::ProcessTrackerError::from)?;

        match move_to_next_target(&mut tracking_data) {
            true => requeue_for_next_target(state, &process.id, &tracking_data)
                .await
                .map_err(errors::ProcessTrackerError::from),
            false => {
                state
                    .store
                    .as_scheduler()
                    .finish_process_with_business_status(process, "COMPLETED_BY_PT")
                    .await
                    .map_err(errors::ProcessTrackerError::from)?;
                info!(
                    job_id = %tracking_data.job_id,
                    "Blocklist profile clone job finished"
                );
                Ok(())
            }
        }
    }

    /// Retries the current target on the copy-retry schedule. Once that is spent, the target is
    /// recorded as failed and the tracker moves on to the next one. If even that fails (unreadable
    /// tracking data, or a missing or malformed job), the job is marked failed as a whole and the
    /// tracker finished, so every job reaches a final status.
    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        error!(
            process_id = %process.id,
            retry_count = process.retry_count,
            ?error,
            "Blocklist clone attempt failed"
        );

        match copy_retry_delay(process.retry_count) {
            Some(delay) => {
                let schedule_time = common_utils::date_time::now()
                    .saturating_add(time::Duration::seconds(i64::from(delay)));
                state
                    .store
                    .as_scheduler()
                    .retry_process(process, schedule_time)
                    .await
                    .change_context(errors::ProcessTrackerError::ProcessUpdateFailed)
            }
            None => match fail_current_target(state, &process).await {
                Ok(true) => Ok(()),
                Ok(false) => state
                    .store
                    .as_scheduler()
                    .finish_process_with_business_status(process, "COMPLETED_BY_PT")
                    .await
                    .change_context(errors::ProcessTrackerError::ProcessUpdateFailed),
                Err(error) => {
                    error!(
                        ?error,
                        process_id = %process.id,
                        "Could not record the failed clone target; failing the whole job"
                    );
                    fail_job(state, &process).await;
                    state
                        .store
                        .as_scheduler()
                        .finish_process_with_business_status(process, "RETRIES_EXCEEDED")
                        .await
                        .change_context(errors::ProcessTrackerError::ProcessUpdateFailed)
                }
            },
        }
    }
}
