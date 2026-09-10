use analytics::{errors::AnalyticsError, lambda_utils::invoke_lambda};
use api_models::analytics::GenerateReportRequest;
use common_utils::errors::CustomResult;
#[cfg(feature = "v1")]
use router_env::logger;

#[cfg(feature = "v1")]
use crate::routes::metrics;
use crate::routes::SessionState;
#[cfg(feature = "v1")]
use crate::types::storage;

pub const PAYMENT_REPORT_TASK: &str = "PAYMENT_REPORT";

pub async fn trigger_payment_report_generation(
    state: &SessionState,
    report_request: GenerateReportRequest,
) -> CustomResult<(), AnalyticsError> {
    #[cfg(feature = "v1")]
    if state
        .conf
        .report_download_config
        .generate_payment_reports_via_scheduler
    {
        return schedule_payment_report_task(state, report_request).await;
    }

    let json_bytes =
        serde_json::to_vec(&report_request).map_err(|_| AnalyticsError::UnknownError)?;
    invoke_lambda(
        &state.conf.report_download_config.payment_function,
        &state.conf.report_download_config.region,
        &json_bytes,
    )
    .await
}

#[cfg(feature = "v1")]
async fn schedule_payment_report_task(
    state: &SessionState,
    report_request: GenerateReportRequest,
) -> CustomResult<(), AnalyticsError> {
    let runner = storage::ProcessTrackerRunner::GenerateReportWorkflow;
    // Reports for the same scope and time range may legitimately be requested multiple
    // times, so the task id is unique per request instead of deterministic.
    let process_tracker_id = format!(
        "{runner}_{PAYMENT_REPORT_TASK}_{}",
        uuid::Uuid::new_v4().simple()
    );

    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        PAYMENT_REPORT_TASK,
        runner,
        ["REPORT", "PAYMENT"],
        report_request,
        None,
        common_utils::date_time::now(),
        common_types::consts::API_VERSION,
        state.conf.application_source,
    )
    .map_err(|error| {
        logger::error!(?error, "Failed to construct payment report task");
        AnalyticsError::UnknownError
    })?;

    state
        .store
        .insert_process(process_tracker_entry)
        .await
        .map_err(|error| {
            logger::error!(?error, "Failed to insert payment report task");
            AnalyticsError::UnknownError
        })?;

    metrics::TASKS_ADDED_COUNT.add(1, router_env::metric_attributes!(("flow", "PaymentReport")));

    Ok(())
}
