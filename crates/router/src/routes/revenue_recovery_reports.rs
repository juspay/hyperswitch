use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::revenue_recovery_reports::RevenueRecoveryReportMetadata;
use futures::StreamExt;
use router_env::{instrument, logger, Flow};

use crate::{
    core::revenue_recovery_reports,
    routes::AppState,
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::RevenueRecoveryReportUpload))]
pub async fn upload_revenue_recovery_report_stream_handler(
    state: web::Data<AppState>,
    req: HttpRequest,
    mut payload: Multipart,
) -> HttpResponse {
    let flow = Flow::RevenueRecoveryReportUpload;

    let auth_result = auth::AdminApiAuthWithMerchantIdFromHeader
        .authenticate_and_fetch(req.headers(), &state)
        .await;

    let auth_data: auth::AuthenticationData = match auth_result {
        Ok((data, _)) => data,
        Err(err) => return api::log_and_return_error_response(err),
    };

    let platform = auth_data.platform;

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
                        chunk_res.map_err(actix_web::error::MultipartError::from)
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

    let session_state = state.get_ref().clone().into();
    match revenue_recovery_reports::upload_revenue_recovery_report_stream(
        session_state,
        platform,
        metadata,
        file_stream,
    )
    .await
    {
        Ok(response) => api::log_and_return_response(response, &flow),
        Err(err) => api::log_and_return_error_response(err),
    }
}
