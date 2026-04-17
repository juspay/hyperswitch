use std::sync::Arc;

use actix_multipart::{Multipart, MultipartError};
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::revenue_recovery_reports::{
    RevenueRecoveryReportMetadata, RevenueRecoveryReportUploadResponse, UploadStatus,
    UploadStatusData,
};
use common_utils::consts::{DEFAULT_TENANT, TENANT_HEADER};
use futures::{StreamExt, TryStreamExt};
use router_env::{instrument, logger, tracing, Flow};

use crate::{
    consts,
    core::revenue_recovery_reports,
    headers,
    routes::app::SessionState,
    routes::AppState,
    services::{api, authentication as auth, ApplicationResponse},
    types::storage::revenue_recovery_reports::RevenueRecoveryUploadStatusManager,
};
use crate::services::authentication::AuthenticateAndFetch;

const UPLOAD_STATUS_TTL_SECONDS: i64 = 86400;

#[instrument(skip_all, fields(flow = ?Flow::RevenueRecoveryReportUpload))]
pub async fn upload_revenue_recovery_report_stream_handler(
    state: web::Data<AppState>,
    req: HttpRequest,
    mut payload: Multipart,
) -> HttpResponse {
    let flow = Flow::RevenueRecoveryReportUpload;
    let app_state = state.get_ref().clone();

    let tenant_id = if !app_state.conf.multitenancy.enabled {
        match common_utils::id_type::TenantId::try_from_string(DEFAULT_TENANT.to_owned()) {
            Ok(id) => id,
            Err(_) => return api::log_and_return_error_response(
                crate::core::errors::ApiErrorResponse::InternalServerError.into(),
            ),
        }
    } else {
        let request_tenant_id = match req
            .headers()
            .get(TENANT_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or(crate::core::errors::ApiErrorResponse::MissingTenantId)
        {
            Ok(h) => h,
            Err(e) => return api::log_and_return_error_response(e.into()),
        };
        let request_tenant_id = match common_utils::id_type::TenantId::try_from_string(
            request_tenant_id.to_string(),
        ) {
            Ok(id) => id,
            Err(_) => {
                return api::log_and_return_error_response(
                    crate::core::errors::ApiErrorResponse::InvalidRequestData {
                        message: format!("`{}` header is invalid", headers::X_TENANT_ID),
                    }
                    .into(),
                )
            }
        };
        match app_state.conf.multitenancy.get_tenant(&request_tenant_id) {
            Some(tenant) => tenant.tenant_id.clone(),
            None => {
                return api::log_and_return_error_response(
                    crate::core::errors::ApiErrorResponse::InvalidTenant {
                        tenant_id: request_tenant_id.get_string_repr().to_string(),
                    }
                    .into(),
                )
            }
        }
    };

    let session_state = match Arc::new(app_state.clone()).get_session_state(
        &tenant_id,
        None,
        || crate::core::errors::ApiErrorResponse::InvalidTenant {
            tenant_id: tenant_id.get_string_repr().to_string(),
        },
    ) {
        Ok(s) => s,
        Err(e) => return api::log_and_return_error_response(e.into()),
    };

    let auth_result = auth::AdminApiAuthWithMerchantIdFromHeader
        .authenticate_and_fetch(req.headers(), &session_state)
        .await;

    let auth_data: auth::AuthenticationData = match auth_result {
        Ok((data, _)) => data,
        Err(err) => return api::log_and_return_error_response(err),
    };

    let platform = auth_data.platform;
    let merchant_id_str = platform
        .get_processor()
        .get_account()
        .get_id()
        .get_string_repr()
        .to_string();

    let mut file_name: Option<String> = None;
    let mut timeline: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut file_content_stream = None;

    while let Ok(Some(field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name();

        match field_name {
            Some("file") => {
                file_name = content_disposition.get_filename().map(String::from);
                content_type = field.content_type().map(|m| m.essence_str().to_string());
                file_content_stream =
                    Some(field.map(|chunk_res| {
                        chunk_res.map_err(MultipartError::from)
                    }));
            }
            Some("timeline") => {
                let mut bytes = web::BytesMut::new();
                let mut stream = field.into_stream();
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => bytes.extend_from_slice(&chunk),
                        Err(err) => {
                            logger::error!("Error reading timeline field: {:?}", err);
                            return HttpResponse::BadRequest().json(serde_json::json!({
                                "error": "Error reading timeline field"
                            }));
                        }
                    }
                }
                timeline = match String::from_utf8(bytes.to_vec()) {
                    Ok(s) => Some(s),
                    Err(err) => {
                        logger::error!("Error decoding timeline to UTF-8: {:?}", err);
                        return HttpResponse::BadRequest().json(serde_json::json!({
                            "error": "Invalid timeline encoding"
                        }));
                    }
                };
            }
            _ => {
                let mut stream = field.into_stream();
                while stream.next().await.is_some() {}
            }
        }
    }

    let extracted_file_name = match file_name {
        Some(name) => name,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Missing 'file' field or filename in multipart form"
            }));
        }
    };

    let extracted_timeline = match timeline {
        Some(t) => t,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Missing 'timeline' field in multipart form"
            }));
        }
    };

    let file_stream = match file_content_stream {
        Some(stream) => stream,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Missing 'file' content stream in multipart form"
            }));
        }
    };

    let metadata = RevenueRecoveryReportMetadata {
        file_name: extracted_file_name,
        timeline: extracted_timeline,
        content_type,
    };

    let file_id = common_utils::generate_id(consts::ID_LENGTH, "rr_report");
    let upload_request_time = time::OffsetDateTime::now_utc();

    let initial_status_data = UploadStatusData {
        file_id: file_id.clone(),
        status: UploadStatus::Uploading,
        s3_key: None,
        error: None,
        uploaded_at: upload_request_time.to_string(),
        completed_at: None,
        merchant_id: merchant_id_str.clone(),
    };

    let session_state_clone = session_state.clone();
    let platform_clone = platform.clone();
    let metadata_clone = metadata.clone();
    let file_id_clone = file_id.clone();
    let initial_status_data_clone = initial_status_data.clone();

    let set_status_result = RevenueRecoveryUploadStatusManager::set_upload_status(
        &session_state,
        &file_id,
        initial_status_data_clone,
        UPLOAD_STATUS_TTL_SECONDS,
    )
    .await;

    if let Err(err) = set_status_result {
        logger::error!("Failed to set initial upload status in Redis: {:?}", err);
        return api::log_and_return_error_response(err);
    }

    actix_web::rt::spawn(async move {
        let _ = revenue_recovery_reports::upload_revenue_recovery_report_background(
            session_state_clone,
            platform_clone,
            metadata_clone,
            file_stream,
            file_id_clone,
            initial_status_data,
        )
        .await;
    });

    HttpResponse::Ok().json(RevenueRecoveryReportUploadResponse {
        file_id: file_id.clone(),
        s3_key: format!(
            "revenue_recovery_reports/{}/{}_{}_{}",
            merchant_id_str, metadata.timeline, file_id, metadata.file_name
        ),
        status: "uploading".to_string(),
        uploaded_at: upload_request_time.to_string(),
        merchant_id: merchant_id_str.clone(),
    })
}

#[instrument(skip_all, fields(flow = ?Flow::RevenueRecoveryReportUpload))]
pub async fn get_revenue_recovery_report_status_handler(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::RevenueRecoveryReportUpload;
    let file_id = path.into_inner();
    let app_state = state.get_ref().clone();

    let tenant_id = if !app_state.conf.multitenancy.enabled {
        match common_utils::id_type::TenantId::try_from_string(DEFAULT_TENANT.to_owned()) {
            Ok(id) => id,
            Err(_) => return api::log_and_return_error_response(
                crate::core::errors::ApiErrorResponse::InternalServerError.into(),
            ),
        }
    } else {
        let request_tenant_id = match req
            .headers()
            .get(TENANT_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or(crate::core::errors::ApiErrorResponse::MissingTenantId)
        {
            Ok(h) => h,
            Err(e) => return api::log_and_return_error_response(e.into()),
        };
        let request_tenant_id = match common_utils::id_type::TenantId::try_from_string(
            request_tenant_id.to_string(),
        ) {
            Ok(id) => id,
            Err(_) => {
                return api::log_and_return_error_response(
                    crate::core::errors::ApiErrorResponse::InvalidRequestData {
                        message: format!("`{}` header is invalid", headers::X_TENANT_ID),
                    }
                    .into(),
                )
            }
        };
        match app_state.conf.multitenancy.get_tenant(&request_tenant_id) {
            Some(tenant) => tenant.tenant_id.clone(),
            None => {
                return api::log_and_return_error_response(
                    crate::core::errors::ApiErrorResponse::InvalidTenant {
                        tenant_id: request_tenant_id.get_string_repr().to_string(),
                    }
                    .into(),
                )
            }
        }
    };

    let session_state = match Arc::new(app_state.clone()).get_session_state(
        &tenant_id,
        None,
        || crate::core::errors::ApiErrorResponse::InvalidTenant {
            tenant_id: tenant_id.get_string_repr().to_string(),
        },
    ) {
        Ok(s) => s,
        Err(e) => return api::log_and_return_error_response(e.into()),
    };

    let auth_result = auth::AdminApiAuthWithMerchantIdFromHeader
        .authenticate_and_fetch(req.headers(), &session_state)
        .await;

    let auth_data: auth::AuthenticationData = match auth_result {
        Ok((data, _)) => data,
        Err(err) => return api::log_and_return_error_response(err),
    };

    let platform = auth_data.platform;

    match revenue_recovery_reports::get_revenue_recovery_report_status(
        session_state,
        platform,
        file_id,
    )
    .await
    {
        Ok(ApplicationResponse::Json(response)) => HttpResponse::Ok().json(response),
        Ok(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Unexpected response type"
        })),
        Err(err) => api::log_and_return_error_response(err),
    }
}
