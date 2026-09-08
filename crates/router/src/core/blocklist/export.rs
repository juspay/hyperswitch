//! Blocklist CSV export helpers.
use api_models::blocklist as api_blocklist;
use common_utils::{date_time, id_type};
use csv::WriterBuilder;
use error_stack::ResultExt;
use router_env::{instrument, tracing};
use scheduler::utils as pt_utils;

use super::batch;
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        utils as core_utils,
    },
    logger,
    routes::SessionState,
    types::{domain, storage},
};

const BLOCKLIST_EXPORT_TASK: &str = "BLOCKLIST_EXPORT";
const BLOCKLIST_EXPORT_TAGS: [&str; 2] = ["BLOCKLIST", "EXPORT"];

pub(crate) const EXPORT_PAGE_SIZE: i64 = 5_000;

/// S3 requires every part except the last to be at least 5 MB.
pub(crate) const MULTIPART_PART_SIZE: usize = 5 * 1024 * 1024;

/// Lifetime of a generated download link. Kept short because the credentials signing it rotate.
const PRESIGNED_URL_TTL: time::Duration = time::Duration::seconds(900);

/// Dated so repeated exports do not overwrite each other in a downloads folder.
fn export_file_name(created_at: time::PrimitiveDateTime) -> String {
    format!(
        "blocklist_export_{:04}{:02}{:02}_{:02}{:02}{:02}.csv",
        created_at.year(),
        u8::from(created_at.month()),
        created_at.day(),
        created_at.hour(),
        created_at.minute(),
        created_at.second(),
    )
}

pub(crate) fn export_key(merchant_id: &str, profile_id: &str, export_id: &str) -> String {
    format!("blocklist/export/{merchant_id}/{profile_id}/{export_id}.csv")
}

/// The header row, matching what `POST /blocklist/batch` accepts.
pub(crate) fn csv_header() -> RouterResult<Vec<u8>> {
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .from_writer(Vec::new());
    writer
        .write_record(batch::CSV_HEADER)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to write blocklist export CSV header")?;
    writer
        .into_inner()
        .map_err(|error| error.into_error())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to finalize blocklist export CSV header")
}

/// Serializes one page of blocklist entries into CSV.
pub(crate) fn rows_to_csv_bytes(rows: &[storage::Blocklist]) -> RouterResult<Vec<u8>> {
    batch::records_to_csv_bytes(
        rows.iter()
            .map(batch::BlocklistCsvRecord::from_stored_entry),
    )
}

#[instrument(skip_all, fields(flow = ?router_env::Flow::CreateBlocklistExport))]
pub async fn initiate_blocklist_export(
    state: &SessionState,
    platform: &domain::Platform,
    profile_id: Option<id_type::ProfileId>,
) -> RouterResult<api_blocklist::BlocklistExportResponse> {
    let processor_merchant_id = platform.get_processor().get_account().get_id();
    let profile_id = core_utils::get_profile_id_from_business_details(
        None,
        None,
        platform.get_processor(),
        profile_id.as_ref(),
        &*state.store,
        true,
    )
    .await?;

    let export_id = common_utils::generate_id(crate::consts::ID_LENGTH, "blkexp");
    let now = date_time::now();
    let file_name = export_file_name(now);

    let job_new = storage::BatchBlocklistJobNew {
        id: export_id.clone(),
        merchant_id: processor_merchant_id.clone(),
        status: common_enums::BatchBlocklistJobStatus::Initiated,
        total_rows: 0,
        succeeded_rows: 0,
        failed_rows: 0,
        created_at: now,
        updated_at: now,
        profile_id: profile_id.clone(),
        job_type: common_enums::BatchBlocklistJobType::Export,
        file_name: Some(file_name.clone()),
    };

    state
        .store
        .insert_batch_blocklist_job(job_new)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::InternalServerError)?;

    let tracking_data = storage::BlocklistExportTrackingData {
        job_id: export_id.clone(),
        merchant_id: platform.get_provider().get_account().get_id().clone(),
        processor_merchant_id: Some(processor_merchant_id.clone()),
        profile_id,
        snapshot_at: now,
        upload_id: None,
    };

    let runner = storage::ProcessTrackerRunner::BlocklistExportWorkflow;
    let process_tracker_id = pt_utils::get_process_tracker_id(
        runner,
        BLOCKLIST_EXPORT_TASK,
        &export_id,
        processor_merchant_id,
    );

    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        BLOCKLIST_EXPORT_TASK,
        runner,
        BLOCKLIST_EXPORT_TAGS,
        tracking_data,
        None,
        date_time::now(),
        common_types::consts::API_VERSION,
        common_enums::ApplicationSource::Main,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to create ProcessTrackerNew for blocklist export job")?;

    state
        .store
        .insert_process(process_tracker_entry)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to enqueue blocklist export ProcessTracker task")?;

    logger::info!(export_id = %export_id, "Blocklist export job initiated");

    Ok(api_blocklist::BlocklistExportResponse {
        export_id,
        status: common_enums::BatchBlocklistJobStatus::Initiated,
        file_name,
    })
}

/// Shared by the listing's `downloadable` flag and the signing path, so the two cannot disagree.
pub(crate) fn is_export_downloadable(job: &storage::BatchBlocklistJob) -> bool {
    matches!(
        job.job_type,
        Some(common_enums::BatchBlocklistJobType::Export)
    ) && matches!(job.status, common_enums::BatchBlocklistJobStatus::Completed)
        && job.file_key.is_some()
        && job
            .expires_at
            .is_none_or(|expires_at| expires_at > date_time::now())
}

/// Logs rather than propagating when the storage backend cannot sign.
pub(crate) async fn presign_export_download(
    state: &SessionState,
    job: &storage::BatchBlocklistJob,
) -> Option<(
    hyperswitch_masking::Secret<url::Url>,
    time::PrimitiveDateTime,
)> {
    match job
        .file_key
        .as_ref()
        .filter(|_| is_export_downloadable(job))
    {
        Some(file_key) => state
            .file_storage_client
            .get_presigned_download_url(
                file_key,
                PRESIGNED_URL_TTL.unsigned_abs(),
                job.file_name.as_deref(),
            )
            .await
            .inspect_err(|error| {
                logger::warn!(
                    ?error,
                    export_id = %job.id,
                    "Failed to sign a blocklist export download URL"
                );
            })
            .ok()
            .map(|url| {
                (
                    hyperswitch_masking::Secret::new(url),
                    date_time::now().saturating_add(PRESIGNED_URL_TTL),
                )
            }),
        None => None,
    }
}
