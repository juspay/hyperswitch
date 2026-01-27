use actix_web::web::Bytes;
use api_models::revenue_recovery_reports::{
    RevenueRecoveryReportMetadata, RevenueRecoveryReportUploadResponse,
};
use common_utils::id_type;
use error_stack::ResultExt;
use futures::{Stream, StreamExt};
use router_env::logger;
use time::format_description;

use crate::{
    consts,
    core::errors::{self, RouterResult},
    routes::SessionState,
    services::ApplicationResponse,
    types::domain,
};

const DEFAULT_S3_CONTENT_TYPE: &str = "text/csv";
const MIN_S3_MULTIPART_PART_SIZE: usize = 5 * 1024 * 1024;

pub async fn upload_revenue_recovery_report_stream(
    state: SessionState,
    platform: domain::Platform,
    metadata: RevenueRecoveryReportMetadata,
    file_stream: impl Stream<Item = Result<Bytes, actix_web::error::MultipartError>> + Unpin,
) -> RouterResult<ApplicationResponse<RevenueRecoveryReportUploadResponse>> {
    let merchant_id: id_type::MerchantId = platform.get_processor().get_account().get_id().clone();
    let merchant_id_str = merchant_id.get_string_repr();

    let file_id = common_utils::generate_id(consts::ID_LENGTH, "rr_report");
    let upload_request_time = time::OffsetDateTime::now_utc();

    let parsed_timeline = time::OffsetDateTime::parse(
        &metadata.timeline,
        &format_description::well_known::Iso8601::DEFAULT,
    )
    .change_context(errors::ApiErrorResponse::InvalidRequestData {
        message: "Invalid 'timeline' format. Expected ISO8601 (e.g., 2024-01-15T10:30:00Z)"
            .to_string(),
    })
    .attach_printable_lazy(|| format!("Failed to parse timeline: {}", metadata.timeline))?;

    let s3_content_type = metadata
        .content_type
        .unwrap_or_else(|| DEFAULT_S3_CONTENT_TYPE.to_string());

    let s3_key = format!(
        "revenue_recovery_reports/{}/{:04}{:02}{:02}T{:02}{:02}{:02}Z_{}_{}",
        merchant_id_str,
        parsed_timeline.year(),
        parsed_timeline.month() as u8,
        parsed_timeline.day(),
        parsed_timeline.hour(),
        parsed_timeline.minute(),
        parsed_timeline.second(),
        file_id,
        metadata.file_name
    );

    logger::info!("Initiating multipart upload for {}", s3_key);

    let upload_id = state
        .file_storage_client
        .initiate_multipart_upload(&s3_key, &s3_content_type)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to initiate S3 multipart upload")?;

    let mut current_part_number = 1;
    let mut uploaded_parts = Vec::new();
    let mut part_buffer = Vec::new();

    futures::pin_mut!(file_stream);

    while let Some(chunk_result) = file_stream.next().await {
        let chunk = chunk_result
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while reading file stream chunk")?;

        part_buffer.extend_from_slice(&chunk);

        if part_buffer.len() >= MIN_S3_MULTIPART_PART_SIZE {
            logger::info!(
                "Uploading part {} for {}. Size: {}",
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
                    aws_sdk_s3::primitives::ByteStream::from(part_buffer.clone()),
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable_lazy(|| {
                    format!("Failed to upload S3 part {}", current_part_number)
                })?;

            uploaded_parts.push(api_models::revenue_recovery_reports::CompletedPart {
                part_number: current_part_number,
                e_tag,
            });

            part_buffer.clear();
            current_part_number += 1;
        }
    }

    if !part_buffer.is_empty() {
        logger::info!(
            "Uploading final part {} for {}. Size: {}",
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
                aws_sdk_s3::primitives::ByteStream::from(part_buffer),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!("Failed to upload final S3 part {}", current_part_number)
            })?;

        uploaded_parts.push(api_models::revenue_recovery_reports::CompletedPart {
            part_number: current_part_number,
            e_tag,
        });
    }

    uploaded_parts.sort_by_key(|p| p.part_number);

    logger::info!(
        "Completing multipart upload for {}. Total parts: {}",
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
        "Successfully uploaded revenue recovery report to {}",
        s3_key
    );

    Ok(ApplicationResponse::Json(
        RevenueRecoveryReportUploadResponse {
            file_id,
            s3_key,
            status: "stored".to_string(),
            uploaded_at: upload_request_time.to_string(),
            merchant_id: merchant_id_str,
        },
    ))
}
