//! Durable batch deletion of payment method locker data.

use api_models::payment_methods as api_pm;
use common_utils::{date_time, ext_traits::ValueExt};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors::{self, RouterResponse, RouterResult},
    routes::SessionState,
    services::ApplicationResponse,
    types::storage,
};

const CLEANUP_TASK: &str = "PAYMENT_METHOD_FINGERPRINT_CLEANUP";
const CLEANUP_TAGS: [&str; 2] = ["PAYMENT_METHOD", "FINGERPRINT_CLEANUP"];
const MAX_CSV_ROWS: usize = 10_000;

#[derive(Debug, Deserialize)]
struct BatchDeletionCsvRecord {
    payment_method_id: String,
}

/// Persisted in `process_tracker.tracking_data`, so progress survives router and scheduler restarts.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct PaymentMethodBatchDeletionTrackingData {
    #[serde(default)]
    pub deletion_type: api_pm::PaymentMethodDeletionType,
    pub payment_method_ids: Vec<String>,
    pub next_index: usize,
    pub successful_ids: usize,
    pub failures: Vec<api_pm::PaymentMethodBatchDeletionFailure>,
}

fn invalid_csv(message: impl Into<String>) -> error_stack::Report<errors::ApiErrorResponse> {
    errors::ApiErrorResponse::InvalidRequestData {
        message: message.into(),
    }
    .into()
}

fn parse_csv(csv_bytes: &[u8]) -> RouterResult<Vec<String>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(csv_bytes);

    let headers =
        reader
            .headers()
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "CSV must include exactly one header: payment_method_id".to_string(),
            })?;
    let header = headers
        .get(0)
        .map(|value| value.trim_start_matches('\u{feff}'));
    if headers.len() != 1 || header != Some("payment_method_id") {
        return Err(invalid_csv(
            "CSV must include exactly one header: payment_method_id",
        ));
    }

    let mut payment_method_ids = Vec::new();
    for (index, record) in reader
        .deserialize::<BatchDeletionCsvRecord>()
        .enumerate()
        .take(MAX_CSV_ROWS + 1)
    {
        let row_number = index + 2;
        let record = record.change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: format!("Invalid CSV row {row_number}"),
        })?;
        let payment_method_id = record.payment_method_id.trim();
        if payment_method_id.is_empty() {
            return Err(invalid_csv(format!(
                "payment_method_id must not be empty at row {row_number}"
            )));
        }
        payment_method_ids.push(payment_method_id.to_string());
    }

    if payment_method_ids.is_empty() {
        return Err(invalid_csv(
            "CSV must contain at least one payment_method_id",
        ));
    }
    if payment_method_ids.len() > MAX_CSV_ROWS {
        return Err(invalid_csv(format!(
            "CSV exceeds the maximum of {MAX_CSV_ROWS} payment_method_ids"
        )));
    }

    Ok(payment_method_ids)
}

fn cleanup_status(process: &storage::ProcessTracker) -> api_pm::PaymentMethodBatchDeletionStatus {
    match process.status {
        common_enums::ProcessTrackerStatus::New
        | common_enums::ProcessTrackerStatus::Pending
        | common_enums::ProcessTrackerStatus::Processing => {
            api_pm::PaymentMethodBatchDeletionStatus::Initiated
        }
        common_enums::ProcessTrackerStatus::ProcessStarted => {
            api_pm::PaymentMethodBatchDeletionStatus::Processing
        }
        common_enums::ProcessTrackerStatus::Finish
            if process.business_status
                == diesel_models::process_tracker::business_status::COMPLETED_BY_PT =>
        {
            api_pm::PaymentMethodBatchDeletionStatus::Completed
        }
        common_enums::ProcessTrackerStatus::Finish | common_enums::ProcessTrackerStatus::Review => {
            api_pm::PaymentMethodBatchDeletionStatus::Failed
        }
    }
}

async fn find_cleanup_process(
    state: &SessionState,
    job_id: &str,
) -> RouterResult<storage::ProcessTracker> {
    let process = state
        .store
        .find_process_by_id(job_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .ok_or_else(|| errors::ApiErrorResponse::GenericNotFoundError {
            message: format!("Payment method batch deletion job `{job_id}` not found"),
        })?;

    let expected_runner = storage::ProcessTrackerRunner::PaymentMethodBatchDeletionWorkflow;
    if process.runner.as_deref() != Some(expected_runner.to_string().as_str()) {
        return Err(errors::ApiErrorResponse::GenericNotFoundError {
            message: format!("Payment method batch deletion job `{job_id}` not found"),
        }
        .into());
    }

    Ok(process)
}

pub async fn initiate(
    state: &SessionState,
    csv_bytes: &[u8],
    deletion_type: api_pm::PaymentMethodDeletionType,
) -> RouterResponse<api_pm::PaymentMethodBatchDeletionResponse> {
    let payment_method_ids = parse_csv(csv_bytes)?;
    let total_ids = payment_method_ids.len();
    let job_id = common_utils::generate_id(crate::consts::ID_LENGTH, "pmfpc");
    let tracking_data = PaymentMethodBatchDeletionTrackingData {
        deletion_type,
        payment_method_ids,
        next_index: 0,
        successful_ids: 0,
        failures: Vec::new(),
    };
    let runner = storage::ProcessTrackerRunner::PaymentMethodBatchDeletionWorkflow;
    let process = storage::ProcessTrackerNew::new(
        job_id.clone(),
        CLEANUP_TASK,
        runner,
        CLEANUP_TAGS,
        tracking_data,
        None,
        date_time::now(),
        common_types::consts::API_VERSION,
        common_enums::ApplicationSource::Main,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to create payment method batch deletion task")?;

    state
        .store
        .insert_process(process)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to enqueue payment method batch deletion task")?;

    Ok(ApplicationResponse::Json(
        api_pm::PaymentMethodBatchDeletionResponse {
            job_id,
            deletion_type,
            total_ids,
            status: api_pm::PaymentMethodBatchDeletionStatus::Initiated,
        },
    ))
}

pub async fn status(
    state: &SessionState,
    job_id: &str,
) -> RouterResponse<api_pm::PaymentMethodBatchDeletionStatusResponse> {
    let process = find_cleanup_process(state, job_id).await?;
    let tracking_data: PaymentMethodBatchDeletionTrackingData = process
        .tracking_data
        .clone()
        .parse_value("PaymentMethodBatchDeletionTrackingData")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok(ApplicationResponse::Json(
        api_pm::PaymentMethodBatchDeletionStatusResponse {
            job_id: job_id.to_string(),
            deletion_type: tracking_data.deletion_type,
            status: cleanup_status(&process),
            total_ids: tracking_data.payment_method_ids.len(),
            processed_ids: tracking_data.next_index,
            successful_ids: tracking_data.successful_ids,
            failed_ids: tracking_data.failures.len(),
        },
    ))
}

pub async fn failed_rows_csv(
    state: &SessionState,
    job_id: &str,
) -> RouterResponse<serde_json::Value> {
    let process = find_cleanup_process(state, job_id).await?;
    let status = cleanup_status(&process);
    if !matches!(
        status,
        api_pm::PaymentMethodBatchDeletionStatus::Completed
            | api_pm::PaymentMethodBatchDeletionStatus::Failed
    ) {
        return Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "The cleanup job has not reached a terminal state".to_string(),
        }
        .into());
    }

    let tracking_data: PaymentMethodBatchDeletionTrackingData = process
        .tracking_data
        .parse_value("PaymentMethodBatchDeletionTrackingData")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(Vec::new());
    writer
        .write_record(["payment_method_id", "failure_reason"])
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    for failure in tracking_data.failures {
        writer
            .serialize(failure)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
    }
    let csv = writer
        .into_inner()
        .map_err(|error| error.into_error())
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok(ApplicationResponse::FileData((csv, mime::TEXT_CSV)))
}
