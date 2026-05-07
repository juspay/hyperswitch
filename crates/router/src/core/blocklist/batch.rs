//! Batch blocklist upload helpers.
use api_models::blocklist as api_blocklist;
use common_utils::{date_time, id_type};
use csv::{ReaderBuilder, Trim, WriterBuilder};
use error_stack::{report, ResultExt};
use futures::future;
use router_env::{instrument, tracing};
use scheduler::utils as pt_utils;
use serde::Deserialize;

use crate::{
    core::errors::{self, RouterResult, StorageErrorExt},
    logger,
    routes::SessionState,
    types::storage,
};

const CHUNK_SIZE: usize = 2_000;
const BATCH_BLOCKLIST_TASK: &str = "BATCH_BLOCKLIST_UPLOAD";
const BATCH_BLOCKLIST_TAGS: [&str; 2] = ["BLOCKLIST", "BATCH"];
const MAX_BATCH_CSV_ROWS: usize = 100_000;

/// Returns the file storage key for the original uploaded CSV.
fn original_input_key(merchant_id: &str, job_id: &str) -> String {
    format!("blocklist/batch/{merchant_id}/{job_id}/original.csv")
}

/// Returns the file storage key for a specific input chunk of a batch job.
pub(crate) fn input_chunk_key(merchant_id: &str, job_id: &str, chunk_idx: u32) -> String {
    format!("blocklist/batch/{merchant_id}/{job_id}/input_chunks/{chunk_idx:03}.csv")
}

#[derive(Debug, Clone)]
pub(crate) struct BlocklistRow {
    pub data_kind: common_enums::BlocklistDataKind,
    pub data: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct BlocklistCsvRecord {
    #[serde(rename = "type")]
    kind: String,
    data: String,
    #[serde(default)]
    metadata: Option<String>,
}

fn parse_metadata(s: &str) -> Option<serde_json::Value> {
    let map: serde_json::Map<String, serde_json::Value> = s
        .split(';')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?.trim().to_string();
            let value = parts.next()?.trim().to_string();
            if key.is_empty() {
                return None;
            }
            Some((key, serde_json::Value::String(value)))
        })
        .collect();
    if map.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(map))
    }
}

impl BlocklistRow {
    fn build_row_error(
        row_index: usize,
        data_kind: common_enums::BlocklistDataKind,
        data: String,
        reason: impl Into<String>,
    ) -> api_blocklist::BlocklistRowError {
        api_blocklist::BlocklistRowError {
            row_index,
            data_kind,
            data,
            reason: reason.into(),
        }
    }

    fn parse_kind(kind: &str) -> Option<common_enums::BlocklistDataKind> {
        match kind {
            "card_bin" => Some(common_enums::BlocklistDataKind::CardBin),
            "extended_card_bin" => Some(common_enums::BlocklistDataKind::ExtendedCardBin),
            "fingerprint" => Some(common_enums::BlocklistDataKind::PaymentMethod),
            _ => None,
        }
    }

    fn from_csv_record(
        row_index: usize,
        record: BlocklistCsvRecord,
    ) -> Result<Self, api_blocklist::BlocklistRowError> {
        let kind = record.kind.to_lowercase();
        let data = record.data;

        let parsed_kind = Self::parse_kind(&kind).ok_or_else(|| {
            Self::build_row_error(
                row_index,
                common_enums::BlocklistDataKind::CardBin,
                data.clone(),
                format!(
                    "unknown type `{kind}`; expected card_bin, extended_card_bin, or fingerprint"
                ),
            )
        })?;

        if data.is_empty() {
            return Err(Self::build_row_error(
                row_index,
                parsed_kind,
                String::new(),
                "data field must not be empty",
            ));
        }

        let format_error = match parsed_kind {
            common_enums::BlocklistDataKind::CardBin => {
                if data.len() == 6 && data.chars().all(|c| c.is_ascii_digit()) {
                    None
                } else {
                    Some("card_bin must be exactly 6 digits")
                }
            }
            common_enums::BlocklistDataKind::ExtendedCardBin => {
                if data.len() == 8 && data.chars().all(|c| c.is_ascii_digit()) {
                    None
                } else {
                    Some("extended_card_bin must be exactly 8 digits")
                }
            }
            common_enums::BlocklistDataKind::PaymentMethod => None,
        };

        if let Some(reason) = format_error {
            return Err(Self::build_row_error(
                row_index,
                parsed_kind,
                data.clone(),
                reason,
            ));
        }

        let metadata_raw = record.metadata.as_deref().filter(|s| !s.is_empty());
        let metadata = match metadata_raw {
            None => None,
            Some(s) => match parse_metadata(s) {
                Some(m) => Some(m),
                None => {
                    return Err(Self::build_row_error(
                        row_index,
                        parsed_kind,
                        data.clone(),
                        "metadata must be in key=value format, separated by semicolons (e.g. reason=fraud;source=manual)",
                    ));
                }
            },
        };

        Ok(Self {
            data_kind: parsed_kind,
            data,
            metadata,
        })
    }
}

/// Parses a user-uploaded CSV into blocklist rows, stopping at the first invalid row.
fn parse_csv(csv_bytes: &[u8]) -> Result<Vec<BlocklistRow>, api_blocklist::BlocklistRowError> {
    let mut csv_reader = ReaderBuilder::new()
        .trim(Trim::All)
        .flexible(true)
        .from_reader(csv_bytes);

    let mut rows = Vec::new();
    for (row_index, result) in csv_reader
        .deserialize::<BlocklistCsvRecord>()
        .enumerate()
        .take(MAX_BATCH_CSV_ROWS + 1)
    {
        match result {
            Ok(record) => rows.push(BlocklistRow::from_csv_record(row_index, record)?),
            Err(error) => {
                return Err(BlocklistRow::build_row_error(
                    row_index,
                    common_enums::BlocklistDataKind::CardBin,
                    String::new(),
                    error.to_string(),
                ))
            }
        }
    }

    Ok(rows)
}

/// Serializes a slice of blocklist rows into headerless CSV bytes for chunk storage.
fn rows_to_csv_bytes(rows: &[BlocklistRow]) -> RouterResult<Vec<u8>> {
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .from_writer(Vec::new());
    for row in rows {
        let metadata_str = row
            .metadata
            .as_ref()
            .map(|m| {
                if let serde_json::Value::Object(map) = m {
                    map.iter()
                        .map(|(k, v)| {
                            let val = match v {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            format!("{k}={val}")
                        })
                        .collect::<Vec<_>>()
                        .join(";")
                } else {
                    String::new()
                }
            })
            .unwrap_or_default();
        let type_str = match row.data_kind {
            common_enums::BlocklistDataKind::CardBin => "card_bin",
            common_enums::BlocklistDataKind::ExtendedCardBin => "extended_card_bin",
            common_enums::BlocklistDataKind::PaymentMethod => "fingerprint",
        };
        writer
            .write_record([type_str, row.data.as_str(), metadata_str.as_str()])
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialize batch blocklist input chunk row")?;
    }

    writer
        .into_inner()
        .map_err(|error| error.into_error())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to finalize batch blocklist input chunk CSV")
}

/// Parses a stored input chunk CSV (no header) into blocklist rows.
pub(crate) fn parse_chunk_csv(csv_bytes: &[u8]) -> RouterResult<Vec<BlocklistRow>> {
    let mut csv_reader = ReaderBuilder::new()
        .has_headers(false)
        .trim(Trim::All)
        .from_reader(csv_bytes);
    let mut rows = Vec::new();

    for (row_index, result) in csv_reader.deserialize::<BlocklistCsvRecord>().enumerate() {
        let record = result
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse batch blocklist input chunk CSV")?;

        let row = BlocklistRow::from_csv_record(row_index, record).map_err(|error| {
            report!(errors::ApiErrorResponse::InternalServerError).attach_printable(format!(
                "Invalid batch blocklist input chunk row: {error:?}"
            ))
        })?;

        rows.push(row);
    }

    Ok(rows)
}

/// Validates CSV size and content, returning parsed rows or an error for the first invalid row.
fn validate_csv(csv_bytes: &[u8]) -> RouterResult<Vec<BlocklistRow>> {
    let rows = parse_csv(csv_bytes).map_err(|row_err| {
        logger::warn!(
            row_index = row_err.row_index,
            "Batch blocklist CSV validation failed"
        );
        let error_json = serde_json::to_string(&row_err)
            .unwrap_or_else(|_| format!("validation error at row {}", row_err.row_index));
        errors::ApiErrorResponse::InvalidRequestData {
            message: error_json,
        }
    })?;

    if rows.is_empty() {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "CSV must contain at least one valid data row".to_string(),
        }
        .into());
    }

    if rows.len() > MAX_BATCH_CSV_ROWS {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "CSV exceeds maximum allowed rows ({MAX_BATCH_CSV_ROWS}); got {}",
                rows.len()
            ),
        }
        .into());
    }

    Ok(rows)
}

/// Validates the CSV, splits it into chunks, uploads them to file storage, and enqueues a process tracker job.
#[instrument(skip_all, fields(flow = ?router_env::Flow::BatchBlocklistUpload))]
pub async fn initiate_batch_blocklist_upload(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    csv_bytes: bytes::Bytes,
) -> RouterResult<api_blocklist::BatchBlocklistUploadResponse> {
    let rows = validate_csv(&csv_bytes)?;
    let total_rows = rows.len();
    let job_id = common_utils::generate_id(crate::consts::ID_LENGTH, "blkbatch");
    let mid_str = merchant_id.get_string_repr().to_owned();
    let original_key = original_input_key(&mid_str, &job_id);
    let chunks: Vec<&[BlocklistRow]> = rows.chunks(CHUNK_SIZE).collect();
    let chunk_total_count = u32::try_from(chunks.len())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Chunk count exceeds u32::MAX")?;

    logger::info!(
        job_id = %job_id,
        total_rows,
        chunk_total_count,
        "Uploading batch blocklist input files to file storage"
    );

    state
        .file_storage_client
        .upload_file(&original_key, csv_bytes.to_vec())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to upload original batch blocklist CSV")?;

    let upload_futures: Vec<_> = chunks
        .iter()
        .enumerate()
        .map(|(idx, chunk_rows)| {
            let chunk_idx =
                u32::try_from(idx).change_context(errors::ApiErrorResponse::InternalServerError);
            let key = chunk_idx.map(|ci| input_chunk_key(&mid_str, &job_id, ci));
            let fs = state.file_storage_client.clone();
            async move {
                let key = key?;
                let chunk_bytes = rows_to_csv_bytes(chunk_rows)?;
                fs.upload_file(&key, chunk_bytes)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable_lazy(|| format!("Failed to upload input chunk {idx}"))
            }
        })
        .collect();

    let results = future::join_all(upload_futures).await;
    for result in results {
        result?;
    }

    logger::info!(
        job_id = %job_id,
        chunk_total_count,
        "Uploaded original CSV and all input chunks"
    );
    let now = date_time::now();

    let total_rows_i32 = i32::try_from(total_rows)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Row count exceeds i32::MAX")?;

    let job_new = storage::BatchBlocklistJobNew {
        id: job_id.clone(),
        merchant_id: merchant_id.clone(),
        status: common_enums::BatchBlocklistJobStatus::Initiated,
        total_rows: total_rows_i32,
        succeeded_rows: 0,
        failed_rows: 0,
        created_at: now,
        updated_at: now,
    };

    state
        .store
        .insert_batch_blocklist_job(job_new)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::InternalServerError)?;

    let tracking_data = storage::BatchBlocklistTrackingData {
        job_id: job_id.clone(),
        merchant_id: merchant_id.clone(),
        chunk_total_count,
        completed_chunks: Vec::new(),
    };

    let runner = storage::ProcessTrackerRunner::BatchBlocklistUpload;
    let process_tracker_id =
        pt_utils::get_process_tracker_id(runner, BATCH_BLOCKLIST_TASK, &job_id, merchant_id);

    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        BATCH_BLOCKLIST_TASK,
        runner,
        BATCH_BLOCKLIST_TAGS,
        tracking_data,
        None,
        date_time::now(),
        common_types::consts::API_VERSION,
        common_enums::ApplicationSource::Main,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to create ProcessTrackerNew for batch blocklist job")?;

    state
        .store
        .insert_process(process_tracker_entry)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to enqueue batch blocklist ProcessTracker task")?;

    logger::info!(
        job_id = %job_id,
        total_rows,
        chunk_total_count,
        "Batch blocklist job initiated"
    );

    Ok(api_blocklist::BatchBlocklistUploadResponse {
        job_id,
        total_rows: u32::try_from(total_rows)
            .change_context(errors::ApiErrorResponse::InternalServerError)?,
        status: common_enums::BatchBlocklistJobStatus::Initiated,
    })
}

/// Bulk-inserts all rows in a single chunk into the blocklist table, returning the count of inserted rows.
pub(crate) async fn process_chunk(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    chunk_idx: u32,
    chunk_rows: Vec<BlocklistRow>,
) -> RouterResult<i32> {
    let now = date_time::now();
    let entries: Vec<storage::BlocklistNew> = chunk_rows
        .iter()
        .map(|row| storage::BlocklistNew {
            merchant_id: merchant_id.to_owned(),
            fingerprint_id: row.data.clone(),
            data_kind: row.data_kind,
            metadata: row.metadata.clone(),
            created_at: now,
        })
        .collect();

    let succeeded = i32::try_from(entries.len())
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    state
        .store
        .bulk_insert_blocklist_entries(entries)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("Bulk insert failed for chunk {chunk_idx}"))?;

    logger::info!(chunk_idx, succeeded, "Bulk inserted batch blocklist chunk");

    Ok(succeeded)
}

/// Fetches the status and row counters for a specific batch blocklist job.
#[instrument(skip_all, fields(flow = ?router_env::Flow::GetBatchBlocklistJobStatus))]
pub async fn get_batch_blocklist_job_status(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    job_id: &str,
) -> RouterResult<api_blocklist::BatchBlocklistJobStatusResponse> {
    let job = state
        .store
        .find_batch_blocklist_job_by_id_merchant_id(job_id, merchant_id.get_string_repr())
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: format!("Batch blocklist job `{job_id}` not found"),
        })?;

    Ok(api_blocklist::BatchBlocklistJobStatusResponse {
        job_id: job.id,
        merchant_id: job.merchant_id.get_string_repr().to_owned(),
        status: job.status,
        total_rows: u32::try_from(job.total_rows)
            .change_context(errors::ApiErrorResponse::InternalServerError)?,
        succeeded_rows: u32::try_from(job.succeeded_rows)
            .change_context(errors::ApiErrorResponse::InternalServerError)?,
        failed_rows: u32::try_from(job.failed_rows)
            .change_context(errors::ApiErrorResponse::InternalServerError)?,
        created_at: job.created_at,
        updated_at: job.updated_at,
    })
}

/// Returns a paginated list of batch blocklist jobs for a merchant along with the total count.
#[instrument(skip_all, fields(flow = ?router_env::Flow::ListBatchBlocklistJobs))]
pub async fn list_batch_blocklist_jobs(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    query: api_blocklist::ListBatchBlocklistJobsQuery,
) -> RouterResult<api_blocklist::ListBatchBlocklistJobsResponse> {
    let limit = i64::from(query.limit.get());
    let offset = i64::from(query.offset.get());

    let (jobs, total_count) = future::try_join(
        state.store.list_batch_blocklist_jobs_by_merchant_id(
            merchant_id.get_string_repr(),
            limit,
            offset,
        ),
        state
            .store
            .count_batch_blocklist_jobs_by_merchant_id(merchant_id.get_string_repr()),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to list batch blocklist jobs")?;

    let count = jobs.len();
    let data = jobs
        .into_iter()
        .map(|job| {
            Ok(api_blocklist::BatchBlocklistJobStatusResponse {
                job_id: job.id,
                merchant_id: job.merchant_id.get_string_repr().to_owned(),
                status: job.status,
                total_rows: u32::try_from(job.total_rows)
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
                succeeded_rows: u32::try_from(job.succeeded_rows)
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
                failed_rows: u32::try_from(job.failed_rows)
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
                created_at: job.created_at,
                updated_at: job.updated_at,
            })
        })
        .collect::<RouterResult<Vec<_>>>()?;

    Ok(api_blocklist::ListBatchBlocklistJobsResponse {
        count,
        total_count,
        data,
    })
}
