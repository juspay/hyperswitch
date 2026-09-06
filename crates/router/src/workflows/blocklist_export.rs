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
        blocklist::export,
        errors::{self, RouterResult},
    },
    logger::{error, info, warn},
    routes::SessionState,
    types::storage,
};

pub struct BlocklistExportWorkflow;

struct ExportProgress {
    row_count: usize,
    buffer: Vec<u8>,
    parts: Vec<(i32, String)>,
}

/// The inputs that stay fixed for one export run.
struct ExportScope<'a> {
    file_key: &'a str,
    upload_id: &'a str,
    merchant_id: &'a id_type::MerchantId,
    profile_id: &'a id_type::ProfileId,
    snapshot_at: time::PrimitiveDateTime,
}

async fn drain_scope(
    state: &SessionState,
    scope: &ExportScope<'_>,
    legacy: bool,
    progress: &mut ExportProgress,
) -> RouterResult<()> {
    // `""` sorts below every stored fingerprint, so the first page starts at the beginning.
    let mut cursor = String::new();

    loop {
        let page =
            match legacy {
                false => state
                    .store
                    .list_blocklist_entries_after_fingerprint_by_processor_merchant_id_profile_id(
                        scope.merchant_id,
                        scope.profile_id,
                        cursor.clone(),
                        scope.snapshot_at,
                        export::EXPORT_PAGE_SIZE,
                    )
                    .await,
                true => {
                    state
                        .store
                        .list_blocklist_entries_after_fingerprint_by_legacy_merchant_id_profile_id(
                            scope.merchant_id,
                            scope.profile_id,
                            cursor.clone(),
                            scope.snapshot_at,
                            export::EXPORT_PAGE_SIZE,
                        )
                        .await
                }
            }
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to read a blocklist page for export")?;

        let next_cursor = page.last().map(|row| row.fingerprint_id.clone());

        match next_cursor {
            None => break,
            Some(last_fingerprint) => {
                progress.row_count += page.len();
                progress
                    .buffer
                    .extend_from_slice(&export::rows_to_csv_bytes(&page)?);
                cursor = last_fingerprint;

                if progress.buffer.len() >= export::MULTIPART_PART_SIZE {
                    upload_buffered_part(state, scope, progress).await?;
                }
            }
        }
    }

    Ok(())
}

/// Ships whatever is buffered as the next part.
async fn upload_buffered_part(
    state: &SessionState,
    scope: &ExportScope<'_>,
    progress: &mut ExportProgress,
) -> RouterResult<()> {
    let part_number = i32::try_from(progress.parts.len() + 1)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Blocklist export exceeded the maximum multipart part count")?;

    let e_tag = state
        .file_storage_client
        .upload_part(
            scope.file_key,
            scope.upload_id,
            part_number,
            std::mem::take(&mut progress.buffer),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Failed to upload blocklist export part {part_number}")
        })?;

    progress.parts.push((part_number, e_tag));
    Ok(())
}

async fn run_export_job(
    state: &SessionState,
    process_id: &str,
    tracking_data: storage::BlocklistExportTrackingData,
    merchant_id: &id_type::MerchantId,
) -> RouterResult<(usize, Option<String>)> {
    let file_key = export::export_key(
        merchant_id.get_string_repr(),
        tracking_data.profile_id.get_string_repr(),
        &tracking_data.job_id,
    );

    // A retry inherits the previous attempt's upload. Discard it so its parts stop being billed.
    if let Some(stale_upload_id) = tracking_data.upload_id.as_ref() {
        if let Err(error) = state
            .file_storage_client
            .abort_multipart_upload(&file_key, stale_upload_id)
            .await
        {
            warn!(
                job_id = %tracking_data.job_id,
                error = ?error,
                "Failed to abort the previous blocklist export upload before retrying"
            );
        }
    }

    let upload_id = state
        .file_storage_client
        .create_multipart_upload(&file_key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to start the blocklist export multipart upload")?;

    persist_upload_id(state, process_id, &tracking_data, &upload_id).await?;

    let scope = ExportScope {
        file_key: &file_key,
        upload_id: &upload_id,
        merchant_id,
        profile_id: &tracking_data.profile_id,
        snapshot_at: tracking_data.snapshot_at,
    };

    let result = collect_and_complete(state, &scope).await;

    match result {
        Ok(outcome) => Ok(outcome),
        Err(error) => {
            if let Err(abort_error) = state
                .file_storage_client
                .abort_multipart_upload(&file_key, &upload_id)
                .await
            {
                warn!(
                    job_id = %tracking_data.job_id,
                    error = ?abort_error,
                    "Failed to abort the blocklist export upload after a failure"
                );
            }
            Err(error)
        }
    }
}

async fn collect_and_complete(
    state: &SessionState,
    scope: &ExportScope<'_>,
) -> RouterResult<(usize, Option<String>)> {
    let mut progress = ExportProgress {
        row_count: 0,
        buffer: export::csv_header()?,
        parts: Vec::new(),
    };

    drain_scope(state, scope, false, &mut progress).await?;
    drain_scope(state, scope, true, &mut progress).await?;

    // The final part is the only one allowed below the minimum size.
    if !progress.buffer.is_empty() {
        upload_buffered_part(state, scope, &mut progress).await?;
    }

    let expiration = state
        .file_storage_client
        .complete_multipart_upload(scope.file_key, scope.upload_id, progress.parts)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to complete the blocklist export multipart upload")?;

    Ok((progress.row_count, expiration))
}

/// Records the upload id on the task so a retry can find and discard the abandoned parts.
async fn persist_upload_id(
    state: &SessionState,
    process_id: &str,
    tracking_data: &storage::BlocklistExportTrackingData,
    upload_id: &str,
) -> RouterResult<()> {
    let updated = storage::BlocklistExportTrackingData {
        upload_id: Some(upload_id.to_owned()),
        ..tracking_data.clone()
    };

    let value = serde_json::to_value(&updated)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialise blocklist export tracking data")?;

    state
        .store
        .as_scheduler()
        .process_tracker_update_process_status_by_ids(
            vec![process_id.to_owned()],
            storage::ProcessTrackerUpdate::Update {
                name: None,
                retry_count: None,
                schedule_time: None,
                tracking_data: Some(value),
                business_status: None,
                status: None,
                updated_at: Some(common_utils::date_time::now()),
            },
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to record the blocklist export upload id")?;

    Ok(())
}

/// Reads the expiry date out of `expiry-date="Fri, 10 Sep 2026 00:00:00 GMT", rule-id="..."`.
fn parse_expiry_date(expiration: Option<String>) -> Option<time::PrimitiveDateTime> {
    expiration
        .as_deref()
        .and_then(|value| value.split("expiry-date=\"").nth(1))
        .and_then(|rest| rest.split('"').next())
        .and_then(parse_http_date)
}

/// An HTTP-date spells the zone `GMT` rather than `+0000`, which RFC 2822 only accepts under its
/// obsolete-zone rules, so fall back to reading the shape directly.
fn parse_http_date(date: &str) -> Option<time::PrimitiveDateTime> {
    let from_rfc_2822 =
        time::OffsetDateTime::parse(date, &time::format_description::well_known::Rfc2822)
            .ok()
            .map(|value| time::PrimitiveDateTime::new(value.date(), value.time()));

    from_rfc_2822.or_else(|| {
        time::format_description::parse(
            "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT",
        )
        .ok()
        .and_then(|description| time::PrimitiveDateTime::parse(date, &description).ok())
    })
}

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for BlocklistExportWorkflow {
    #[instrument(skip_all, fields(flow = ?router_env::Flow::CreateBlocklistExport))]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;

        let tracking_data: storage::BlocklistExportTrackingData = process
            .tracking_data
            .clone()
            .parse_value("BlocklistExportTrackingData")
            .map_err(errors::ProcessTrackerError::from)?;

        let job_id = tracking_data.job_id.clone();
        let profile_id = tracking_data.profile_id.clone();
        let merchant_id = tracking_data
            .processor_merchant_id
            .clone()
            .unwrap_or_else(|| tracking_data.merchant_id.clone());
        let merchant_id_str = merchant_id.get_string_repr();

        db.update_batch_blocklist_job_by_id_merchant_id(
            &job_id,
            merchant_id_str,
            storage::BatchBlocklistJobUpdate {
                status: Some(common_enums::BatchBlocklistJobStatus::Processing),
                succeeded_rows: None,
                failed_rows: None,
                total_rows: None,
                file_key: None,
                error_message: None,
                expires_at: None,
                updated_at: common_utils::date_time::now(),
            },
        )
        .await
        .map_err(errors::ProcessTrackerError::from)?;

        let result = run_export_job(state, &process.id, tracking_data, &merchant_id).await;

        match result {
            Ok((row_count, expiration)) => {
                let total_rows = i32::try_from(row_count)
                    .map_err(|_| errors::ProcessTrackerError::UnexpectedFlow)?;

                db.update_batch_blocklist_job_by_id_merchant_id(
                    &job_id,
                    merchant_id_str,
                    storage::BatchBlocklistJobUpdate {
                        status: Some(common_enums::BatchBlocklistJobStatus::Completed),
                        succeeded_rows: Some(total_rows),
                        failed_rows: None,
                        total_rows: Some(total_rows),
                        file_key: Some(export::export_key(
                            merchant_id_str,
                            profile_id.get_string_repr(),
                            &job_id,
                        )),
                        error_message: None,
                        expires_at: parse_expiry_date(expiration),
                        updated_at: common_utils::date_time::now(),
                    },
                )
                .await
                .map_err(errors::ProcessTrackerError::from)?;

                info!(
                    job_id = %job_id,
                    row_count,
                    "Blocklist export completed"
                );

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
                    "Blocklist export failed (retry_count={})",
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
                            "Blocklist export exceeded max retries, marking failed"
                        );
                        db.update_batch_blocklist_job_by_id_merchant_id(
                            &job_id,
                            merchant_id_str,
                            storage::BatchBlocklistJobUpdate {
                                status: Some(common_enums::BatchBlocklistJobStatus::Failed),
                                succeeded_rows: None,
                                failed_rows: None,
                                total_rows: None,
                                file_key: None,
                                error_message: Some(
                                    "Export failed after repeated attempts. Please retry."
                                        .to_string(),
                                ),
                                expires_at: None,
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

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}
