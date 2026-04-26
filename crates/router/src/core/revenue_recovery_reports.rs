use actix_multipart::MultipartError;
use actix_web::web::Bytes;
use api_models::revenue_recovery_reports::{
    RevenueRecoveryReportMetadata, RevenueRecoveryReportStatusResponse, UploadStatus,
    UploadStatusData,
};
use error_stack::ResultExt;
use external_services::file_storage::CompletedPart as StorageCompletedPart;
use futures::{Stream, StreamExt};
use router_env::logger;
use time::format_description;

use crate::{
    core::errors::{self, RouterResult},
    routes::SessionState,
    services::ApplicationResponse,
    types::{domain, storage::revenue_recovery_reports::RevenueRecoveryUploadStatusManager},
};
const DEFAULT_S3_CONTENT_TYPE: &str = "text/csv";
/// Minimum multipart chunk size in bytes (5 MiB).
const MIN_S3_MULTIPART_PART_SIZE: usize = 5 * 1024 * 1024;
/// Maximum allowed time for a background upload before it is marked as failed.
const MAX_UPLOAD_DURATION_SECONDS: u64 = 3 * 60 * 60;
/// Status retention window in seconds (24 hours) to allow polling even after long-running uploads complete.
const UPLOAD_STATUS_TTL_SECONDS: i64 = 86400;

pub async fn upload_revenue_recovery_report_background(
    state: SessionState,
    platform: domain::Platform,
    metadata: RevenueRecoveryReportMetadata,
    mut file_stream: impl Stream<Item = Result<Bytes, MultipartError>> + Unpin,
    file_id: String,
    initial_status_data: UploadStatusData,
) -> RouterResult<()> {
    let merchant_id_str = platform
        .get_processor()
        .get_account()
        .get_id()
        .get_string_repr()
        .to_string();
    let parsed_timeline = time::OffsetDateTime::parse(
        &metadata.timeline,
        &format_description::well_known::Iso8601::DEFAULT,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to parse timeline in background task")?;

    let s3_content_type = metadata
        .content_type
        .unwrap_or_else(|| DEFAULT_S3_CONTENT_TYPE.to_string());

    let s3_key = format!(
        "revenue_recovery_reports/{}/{:04}{:02}{:02}T{:02}{:02}{:02}Z_{}_{}",
        merchant_id_str,
        parsed_timeline.year(),
        u8::from(parsed_timeline.month()),
        parsed_timeline.day(),
        parsed_timeline.hour(),
        parsed_timeline.minute(),
        parsed_timeline.second(),
        file_id,
        metadata.file_name
    );

    logger::info!("Background: Initiating multipart upload for {}", s3_key);

    let mut current_status_data = initial_status_data;
    current_status_data.s3_key = Some(s3_key.clone());

    let result: RouterResult<()> = match tokio::time::timeout(
        std::time::Duration::from_secs(MAX_UPLOAD_DURATION_SECONDS),
        async {
            let upload_id = state
                .file_storage_client
                .initiate_multipart_upload(&s3_key, &s3_content_type)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to initiate S3 multipart upload")?;

            let mut current_part_number = 1;
            let mut uploaded_parts = Vec::new();
            let mut part_buffer = Vec::new();

            while let Some(chunk_result) = file_stream.next().await {
                let chunk = chunk_result
                    .map_err(|_| {
                        error_stack::Report::from(errors::ApiErrorResponse::InternalServerError)
                    })
                    .attach_printable("Error while reading file stream chunk in background")?;

                part_buffer.extend_from_slice(&chunk);

                if part_buffer.len() >= MIN_S3_MULTIPART_PART_SIZE {
                    logger::info!(
                        "Background: Uploading part {} for {}. Size: {}",
                        current_part_number,
                        s3_key,
                        part_buffer.len()
                    );
                    let e_tag = state
                        .file_storage_client
                        .upload_part(
                            &s3_key,
                            &upload_id,
                            current_part_number,
                            part_buffer.clone(),
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable_lazy(|| {
                            format!("Failed to upload S3 part {}", current_part_number)
                        })?;

                    uploaded_parts.push(StorageCompletedPart {
                        part_number: current_part_number,
                        e_tag,
                    });

                    part_buffer.clear();
                    current_part_number += 1;
                }
            }

            if !part_buffer.is_empty() {
                logger::info!(
                    "Background: Uploading final part {} for {}. Size: {}",
                    current_part_number,
                    s3_key,
                    part_buffer.len()
                );
                let e_tag = state
                    .file_storage_client
                    .upload_part(&s3_key, &upload_id, current_part_number, part_buffer)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable_lazy(|| {
                        format!("Failed to upload final S3 part {}", current_part_number)
                    })?;

                uploaded_parts.push(StorageCompletedPart {
                    part_number: current_part_number,
                    e_tag,
                });
            }

            uploaded_parts.sort_by_key(|p| p.part_number);

            logger::info!(
                "Background: Completing multipart upload for {}. Total parts: {}",
                s3_key,
                uploaded_parts.len()
            );
            state
                .file_storage_client
                .complete_multipart_upload(&s3_key, &upload_id, uploaded_parts)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to complete S3 multipart upload")?;

            logger::info!(
                "Background: Successfully uploaded revenue recovery report to {}",
                s3_key
            );
            Ok(())
        },
    )
    .await
    {
        Ok(upload_result) => upload_result,
        Err(_) => {
            logger::error!(
                "Background upload timed out for file_id {} after {} seconds",
                file_id,
                MAX_UPLOAD_DURATION_SECONDS
            );
            current_status_data.status = UploadStatus::Failed;
            current_status_data.error = Some(format!(
                "Upload timed out after {} seconds",
                MAX_UPLOAD_DURATION_SECONDS
            ));
            current_status_data.completed_at = Some(time::OffsetDateTime::now_utc().to_string());
            let _ = RevenueRecoveryUploadStatusManager::set_upload_status(
                &state,
                &file_id,
                current_status_data,
                UPLOAD_STATUS_TTL_SECONDS,
            )
            .await;
            return Ok(());
        }
    };

    match result {
        Ok(_) => {
            current_status_data.status = UploadStatus::Completed;
            current_status_data.completed_at = Some(time::OffsetDateTime::now_utc().to_string());
            let _ = RevenueRecoveryUploadStatusManager::set_upload_status(
                &state,
                &file_id,
                current_status_data,
                UPLOAD_STATUS_TTL_SECONDS,
            )
            .await;
        }
        Err(e) => {
            logger::error!("Background upload failed for file_id {}: {:?}", file_id, e);
            current_status_data.status = UploadStatus::Failed;
            current_status_data.error = Some(e.to_string());
            let _ = RevenueRecoveryUploadStatusManager::set_upload_status(
                &state,
                &file_id,
                current_status_data,
                UPLOAD_STATUS_TTL_SECONDS,
            )
            .await;
        }
    }
    Ok(())
}

pub async fn get_revenue_recovery_report_status(
    state: SessionState,
    platform: domain::Platform,
    file_id: String,
) -> RouterResult<ApplicationResponse<RevenueRecoveryReportStatusResponse>> {
    let merchant_id_str = platform
        .get_processor()
        .get_account()
        .get_id()
        .get_string_repr()
        .to_string();

    let status_data = RevenueRecoveryUploadStatusManager::get_upload_status(&state, &file_id)
        .await?
        .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
            message: format!("Upload status not found for file_id: {}", file_id),
        })?;

    if status_data.merchant_id != merchant_id_str {
        return Err(errors::ApiErrorResponse::Unauthorized.into());
    }

    Ok(ApplicationResponse::Json(
        RevenueRecoveryReportStatusResponse {
            file_id: status_data.file_id,
            status: status_data.status,
            s3_key: status_data.s3_key,
            error: status_data.error,
            uploaded_at: status_data.uploaded_at,
            completed_at: status_data.completed_at,
            merchant_id: status_data.merchant_id,
        },
    ))
}
