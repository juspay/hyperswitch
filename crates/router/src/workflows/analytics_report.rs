use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use router_env::{instrument, tracing};
use scheduler::{
    consumer::{self, types::process_data},
    utils as pt_utils,
    workflows::ProcessTrackerWorkflow,
};

use crate::{
    core::{
        analytics_report::{self, AnalyticsReportTrackingData},
        errors,
    },
    logger::{error, info, warn},
    routes::SessionState,
    types::storage,
};

pub struct AnalyticsReportWorkflow;

/// Builds the report file, discarding any upload a previous attempt abandoned.
async fn generate(
    state: &SessionState,
    process_id: &str,
    tracking_data: &mut AnalyticsReportTrackingData,
) -> errors::CustomResult<(), analytics::errors::AnalyticsError> {
    if let Some(stale_upload_id) = tracking_data.upload_id.take() {
        if let Err(error) = state
            .file_storage_client
            .abort_multipart_upload(&tracking_data.file_key, &stale_upload_id)
            .await
        {
            warn!(
                job_id = %tracking_data.job_id,
                ?error,
                "Failed to abort the previous analytics report upload before retrying"
            );
        }
    }

    let upload_id = state
        .file_storage_client
        .create_multipart_upload(&tracking_data.file_key)
        .await
        .change_context(analytics::errors::AnalyticsError::UnknownError)
        .attach_printable("Failed to start the analytics report upload")?;
    tracking_data.upload_id = Some(upload_id.clone());
    analytics_report::persist_tracking_data(state, process_id, tracking_data).await?;

    match analytics_report::generate_report_file(state, tracking_data, &upload_id).await {
        Ok(row_count) => {
            tracking_data.row_count = Some(row_count);
            tracking_data.upload_id = None;
            // The stage lives in `row_count`, never in the business status: the consumer drops
            // any task whose business status is not `Pending`, so its retry would never run.
            analytics_report::persist_tracking_data(state, process_id, tracking_data).await?;
            info!(job_id = %tracking_data.job_id, row_count, "Analytics report generated");
            Ok(())
        }
        Err(report_error) => {
            match state
                .file_storage_client
                .abort_multipart_upload(&tracking_data.file_key, &upload_id)
                .await
            {
                Ok(()) => tracking_data.upload_id = None,
                Err(error) => warn!(
                    job_id = %tracking_data.job_id,
                    ?error,
                    "Failed to abort the analytics report upload after a failure"
                ),
            }
            Err(report_error)
        }
    }
}

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for AnalyticsReportWorkflow {
    #[instrument(skip_all)]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;

        let mut tracking_data: AnalyticsReportTrackingData = process
            .tracking_data
            .clone()
            .parse_value("AnalyticsReportTrackingData")
            .map_err(errors::ProcessTrackerError::from)?;
        let was_generated = tracking_data.row_count.is_some();

        let result = match was_generated {
            true => Ok(()),
            false => generate(state, &process.id, &mut tracking_data).await,
        };
        let result = match result {
            Ok(()) => {
                analytics_report::deliver_report(state, &process.id, &mut tracking_data).await
            }
            Err(error) => Err(error),
        };

        match result {
            Ok(()) => {
                info!(job_id = %tracking_data.job_id, "Analytics report delivered");
                db.as_scheduler()
                    .finish_process_with_business_status(process, "COMPLETED_BY_PT")
                    .await
                    .map_err(Into::<errors::ProcessTrackerError>::into)?;
            }
            Err(error) => {
                let retry_count = process.retry_count;
                error!(
                    job_id = %tracking_data.job_id,
                    ?error,
                    "Analytics report failed (retry_count={})",
                    retry_count
                );

                let mapping = process_data::RetryMapping::default();
                let time_delta = if retry_count == 0 {
                    Some(mapping.start_after)
                } else {
                    pt_utils::get_delay(retry_count + 1, &mapping.frequencies)
                };

                match pt_utils::get_time_from_delta(time_delta) {
                    Some(schedule_time) => {
                        // Carry this attempt's progress (upload id, file, reached recipients) into
                        // the retry; `retry_process` keeps the stored tracking data.
                        analytics_report::persist_tracking_data(state, &process.id, &tracking_data)
                            .await
                            .map_err(|_| errors::ProcessTrackerError::UnexpectedFlow)?;
                        db.as_scheduler()
                            .retry_process(process, schedule_time)
                            .await
                            .map_err(Into::<errors::ProcessTrackerError>::into)?;
                    }
                    None => {
                        warn!(
                            job_id = %tracking_data.job_id,
                            "Analytics report exceeded max retries"
                        );
                        // A delivery failure means the endpoint is the problem; telling it again
                        // that the report failed would fail the same way.
                        if tracking_data.row_count.is_none() {
                            if let Err(error) =
                                analytics_report::notify_report_failed(state, &tracking_data).await
                            {
                                warn!(
                                    job_id = %tracking_data.job_id,
                                    ?error,
                                    "Failed to send the report_generation.failed webhook"
                                );
                            }
                        }
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

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}
