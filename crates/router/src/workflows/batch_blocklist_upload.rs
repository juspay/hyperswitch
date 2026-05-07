use common_utils::{ext_traits::ValueExt, id_type};
use error_stack::ResultExt;
use router_env::{instrument, tracing};
use scheduler::{
    consumer::{self, types::process_data},
    utils as pt_utils,
    workflows::ProcessTrackerWorkflow,
};

use crate::{
    core::{
        blocklist::batch,
        errors::{self, RouterResult},
    },
    logger::{error, info, warn},
    routes::SessionState,
    types::storage,
};

pub struct BatchBlocklistUploadWorkflow;

/// Processes all unfinished chunks for a batch blocklist job, inserting entries and updating progress after each chunk.
async fn run_batch_job(
    state: &SessionState,
    process_id: &str,
    mut tracking_data: storage::BatchBlocklistTrackingData,
    merchant_id_obj: &id_type::MerchantId,
) -> RouterResult<(i32, i32)> {
    let db = &*state.store;
    let job_id = &tracking_data.job_id;
    let merchant_id_str = merchant_id_obj.get_string_repr();
    let n_chunks = tracking_data.chunk_total_count;

    // Seed from persisted count so retries accumulate correctly.
    let existing_job = db
        .find_batch_blocklist_job_by_id_merchant_id(job_id, merchant_id_str)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch job for counter seeding")?;
    let total_rows = existing_job.total_rows;
    let mut total_succeeded: i32 = existing_job.succeeded_rows;

    for chunk_idx in 0..n_chunks {
        if tracking_data.completed_chunks.contains(&chunk_idx) {
            continue;
        }

        let input_key = batch::input_chunk_key(merchant_id_str, job_id, chunk_idx);
        let chunk_bytes = state
            .file_storage_client
            .retrieve_file(&input_key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!("Failed to retrieve input chunk {chunk_idx} for job {job_id}")
            })?;

        let chunk_rows = batch::parse_chunk_csv(&chunk_bytes)?;

        let chunk_succeeded =
            batch::process_chunk(state, merchant_id_obj, chunk_idx, chunk_rows).await?;
        total_succeeded += chunk_succeeded;

        tracking_data.completed_chunks.push(chunk_idx);
        let td_value = serde_json::to_value(&tracking_data)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialise updated tracking_data")?;

        db.as_scheduler()
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
            .attach_printable_lazy(|| {
                format!("Failed to update tracking_data after chunk {chunk_idx}")
            })?;

        db.update_batch_blocklist_job_by_id_merchant_id(
            job_id,
            merchant_id_str,
            storage::BatchBlocklistJobUpdate {
                status: None,
                succeeded_rows: Some(total_succeeded),
                failed_rows: None,
                updated_at: common_utils::date_time::now(),
            },
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Failed to update progress counters after chunk {chunk_idx}")
        })?;

        info!(
            job_id = %job_id,
            chunk_idx,
            n_chunks,
            completed_chunks = tracking_data.completed_chunks.len(),
            chunk_succeeded,
            "Processed batch blocklist chunk"
        );
    }

    Ok((total_succeeded, total_rows))
}

/// Deletes all input chunk files from file storage after a job completes.
async fn delete_input_chunks(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    job_id: &str,
    chunk_total_count: u32,
) -> RouterResult<()> {
    for chunk_idx in 0..chunk_total_count {
        let input_key = batch::input_chunk_key(merchant_id.get_string_repr(), job_id, chunk_idx);
        state
            .file_storage_client
            .delete_file(&input_key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!("Failed to delete input chunk {chunk_idx} for job {job_id}")
            })?;
    }

    Ok(())
}

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for BatchBlocklistUploadWorkflow {
    /// Deserializes tracking data, runs all pending chunks, then marks the job completed or schedules a retry on failure.
    #[instrument(skip_all, fields(flow = ?router_env::Flow::BatchBlocklistUpload))]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;

        let tracking_data: storage::BatchBlocklistTrackingData = process
            .tracking_data
            .clone()
            .parse_value("BatchBlocklistTrackingData")
            .map_err(errors::ProcessTrackerError::from)?;

        let job_id = tracking_data.job_id.clone();
        let chunk_total_count = tracking_data.chunk_total_count;

        let merchant_id = tracking_data.merchant_id.clone();
        let merchant_id_str = merchant_id.get_string_repr();

        if tracking_data.completed_chunks.is_empty() {
            db.update_batch_blocklist_job_by_id_merchant_id(
                &job_id,
                merchant_id_str,
                storage::BatchBlocklistJobUpdate {
                    status: Some(common_enums::BatchBlocklistJobStatus::Processing),
                    succeeded_rows: None,
                    failed_rows: None,
                    updated_at: common_utils::date_time::now(),
                },
            )
            .await
            .map_err(errors::ProcessTrackerError::from)?;
        }

        let result = run_batch_job(state, &process.id, tracking_data, &merchant_id).await;

        match result {
            Ok((succeeded_rows, total_rows)) => {
                let failed_rows = total_rows - succeeded_rows;
                db.update_batch_blocklist_job_by_id_merchant_id(
                    &job_id,
                    merchant_id_str,
                    storage::BatchBlocklistJobUpdate {
                        status: Some(common_enums::BatchBlocklistJobStatus::Completed),
                        succeeded_rows: Some(succeeded_rows),
                        failed_rows: Some(failed_rows),
                        updated_at: common_utils::date_time::now(),
                    },
                )
                .await
                .map_err(errors::ProcessTrackerError::from)?;

                if let Err(error) =
                    delete_input_chunks(state, &merchant_id, &job_id, chunk_total_count).await
                {
                    warn!(
                        job_id = %job_id,
                        error = ?error,
                        "Failed to clean up batch blocklist input chunks after completion"
                    );
                }

                db.as_scheduler()
                    .finish_process_with_business_status(process, "COMPLETED_BY_PT")
                    .await
                    .map_err(Into::<errors::ProcessTrackerError>::into)?;
            }
            Err(err) => {
                let retry_count = process.retry_count;
                error!(
                    job_id = %job_id,
                    error = ?err,
                    "Batch blocklist processing failed (retry_count={})",
                    retry_count
                );

                let mapping = process_data::RetryMapping::default();
                let time_delta = if retry_count == 0 {
                    Some(mapping.start_after)
                } else {
                    pt_utils::get_delay(retry_count + 1, &mapping.frequencies)
                };
                let schedule_time = pt_utils::get_time_from_delta(time_delta);

                match schedule_time {
                    Some(s_time) => {
                        db.as_scheduler()
                            .retry_process(process, s_time)
                            .await
                            .map_err(Into::<errors::ProcessTrackerError>::into)?;
                    }
                    None => {
                        warn!(
                            job_id = %job_id,
                            "Batch blocklist job exceeded max retries, marking failed"
                        );
                        let current_job = db
                            .find_batch_blocklist_job_by_id_merchant_id(&job_id, merchant_id_str)
                            .await
                            .map_err(errors::ProcessTrackerError::from)?;
                        let failed_rows = current_job.total_rows - current_job.succeeded_rows;
                        db.update_batch_blocklist_job_by_id_merchant_id(
                            &job_id,
                            merchant_id_str,
                            storage::BatchBlocklistJobUpdate {
                                status: Some(common_enums::BatchBlocklistJobStatus::Failed),
                                succeeded_rows: None,
                                failed_rows: Some(failed_rows),
                                updated_at: common_utils::date_time::now(),
                            },
                        )
                        .await
                        .map_err(errors::ProcessTrackerError::from)?;

                        db.as_scheduler()
                            .finish_process_with_business_status(process, "RETRIES_EXCEEDED")
                            .await
                            .map_err(Into::<errors::ProcessTrackerError>::into)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Delegates to the standard consumer error handler to reschedule or mark the process as failed.
    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}
