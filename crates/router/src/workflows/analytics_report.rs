use common_utils::ext_traits::ValueExt;
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
    routes::{metrics, SessionState},
    types::storage::{self, business_status},
};

pub struct AnalyticsReportWorkflow;

/// Writes the file unless an earlier attempt already did, then delivers it.
async fn run(
    state: &SessionState,
    process_id: &str,
    tracking_data: &mut AnalyticsReportTrackingData,
) -> errors::CustomResult<(), analytics::errors::AnalyticsError> {
    if !tracking_data.generated {
        let started = std::time::Instant::now();
        let row_count = analytics_report::generate_report_file(state, tracking_data).await?;
        metrics::ANALYTICS_REPORT_GENERATION_DURATION.record(
            started.elapsed().as_secs_f64(),
            router_env::metric_attributes!(("report", tracking_data.report.definition().name)),
        );
        tracking_data.generated = true;
        analytics_report::persist_tracking_data(state, process_id, tracking_data).await?;
        info!(job_id = %tracking_data.job_id, row_count, "Analytics report generated");
    }
    analytics_report::deliver_report(state, tracking_data).await
}

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for AnalyticsReportWorkflow {
    #[instrument(skip_all)]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let scheduler = state.store.as_scheduler();
        let mut tracking_data: AnalyticsReportTrackingData = process
            .tracking_data
            .clone()
            .parse_value("AnalyticsReportTrackingData")?;

        let Err(error) = run(state, &process.id, &mut tracking_data).await else {
            info!(job_id = %tracking_data.job_id, "Analytics report delivered");
            metrics::ANALYTICS_REPORTS_DELIVERED.add(
                1,
                router_env::metric_attributes!(("report", tracking_data.report.definition().name)),
            );
            scheduler
                .finish_process_with_business_status(process, business_status::COMPLETED_BY_PT)
                .await?;
            return Ok(());
        };
        error!(job_id = %tracking_data.job_id, ?error, retry_count = process.retry_count, "Analytics report failed");

        let mapping = process_data::RetryMapping::default();
        let delay = match process.retry_count {
            0 => Some(mapping.start_after),
            retry_count => pt_utils::get_delay(retry_count + 1, &mapping.frequencies),
        };
        match pt_utils::get_time_from_delta(delay) {
            Some(schedule_time) => scheduler.retry_process(process, schedule_time).await?,
            None => {
                let stage = if tracking_data.generated {
                    "delivery"
                } else {
                    "generation"
                };
                metrics::ANALYTICS_REPORTS_FAILED.add(
                    1,
                    router_env::metric_attributes!(
                        ("report", tracking_data.report.definition().name),
                        ("stage", stage),
                    ),
                );
                // A failed delivery means the endpoint is the problem; telling it again that the
                // report failed would fail the same way.
                if !tracking_data.generated {
                    if let Err(error) =
                        analytics_report::notify_report_failed(state, &tracking_data).await
                    {
                        warn!(job_id = %tracking_data.job_id, ?error, "Failed to send the report_generation.failed webhook");
                    }
                }
                scheduler
                    .finish_process_with_business_status(process, business_status::RETRIES_EXCEEDED)
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
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}
