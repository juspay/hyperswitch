use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::blocklist as api_blocklist;
use error_stack::report;
use futures::{StreamExt, TryStreamExt};
use router_env::Flow;

use crate::{
    core::{api_locking, blocklist, errors},
    routes::AppState,
    services::{api, authentication as auth, authorization::permissions::Permission},
};

#[utoipa::path(
    post,
    path = "/blocklist",
    request_body = BlocklistRequest,
    responses(
        (status = 200, description = "Fingerprint Blocked", body = BlocklistResponse),
        (status = 400, description = "Invalid Data")
    ),
    tag = "Blocklist",
    operation_id = "Block a Fingerprint",
    security(("api_key" = []))
)]
pub async fn add_entry_to_blocklist(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_blocklist::AddToBlocklistRequest>,
) -> HttpResponse {
    let flow = Flow::AddToBlocklist;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, body, _| {
            blocklist::add_entry_to_blocklist(state, auth.platform, body)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: false,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountWrite,
                allow_connected: false,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[utoipa::path(
    delete,
    path = "/blocklist",
    request_body = BlocklistRequest,
    responses(
        (status = 200, description = "Fingerprint Unblocked", body = BlocklistResponse),
        (status = 400, description = "Invalid Data")
    ),
    tag = "Blocklist",
    operation_id = "Unblock a Fingerprint",
    security(("api_key" = []))
)]
pub async fn remove_entry_from_blocklist(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_blocklist::DeleteFromBlocklistRequest>,
) -> HttpResponse {
    let flow = Flow::DeleteFromBlocklist;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, body, _| {
            blocklist::remove_entry_from_blocklist(state, auth.platform, body)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: false,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountWrite,
                allow_connected: false,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[utoipa::path(
    get,
    path = "/blocklist",
    params (
        ("data_kind" = BlocklistDataKind, Query, description = "Kind of the fingerprint list requested"),
    ),
    responses(
        (status = 200, description = "Blocked Fingerprints", body = ListBlocklistResponse),
        (status = 400, description = "Invalid Data")
    ),
    tag = "Blocklist",
    operation_id = "List Blocked fingerprints of a particular kind",
    security(("api_key" = []))
)]
pub async fn list_blocked_payment_methods(
    state: web::Data<AppState>,
    req: HttpRequest,
    query_payload: web::Query<api_blocklist::ListBlocklistQuery>,
) -> HttpResponse {
    let flow = Flow::ListBlocklist;
    let payload = query_payload.into_inner();

    let api_auth = auth::ApiKeyAuth {
        allow_connected_scope_operation: false,
        allow_platform_self_operation: false,
    };

    let (auth_type, _) = match auth::check_sdk_auth_and_get_auth(req.headers(), &payload, api_auth)
    {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(report!(err)),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, mut query, _| {
            if let Some(client_secret) = auth.client_secret {
                query.client_secret = Some(client_secret);
            }

            blocklist::list_blocklist_entries(state, auth.platform, query)
        },
        auth::auth_type(
            &*auth_type,
            &auth::JWTAuth {
                permission: Permission::MerchantAccountRead,
                allow_connected: false,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[utoipa::path(
    post,
    path = "/blocklist/toggle",
    params (
        ("status" = bool, Query, description = "Boolean value to enable/disable blocklist"),
    ),
    responses(
        (status = 200, description = "Blocklist guard enabled/disabled", body = ToggleBlocklistResponse),
        (status = 400, description = "Invalid Data")
    ),
    tag = "Blocklist",
    operation_id = "Toggle blocklist guard for a particular merchant",
    security(("api_key" = []))
)]
pub async fn toggle_blocklist_guard(
    state: web::Data<AppState>,
    req: HttpRequest,
    query_payload: web::Query<api_blocklist::ToggleBlocklistQuery>,
) -> HttpResponse {
    let flow = Flow::ListBlocklist;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        query_payload.into_inner(),
        |state, auth: auth::AuthenticationData, query, _| {
            blocklist::toggle_blocklist_guard(state, auth.platform, query)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: false,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountWrite,
                allow_connected: false,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

// ---- Batch blocklist upload route handlers ----

#[utoipa::path(
    post,
    path = "/blocklist/batch",
    request_body(content = String, content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "Batch blocklist job initiated", body = BatchBlocklistUploadResponse),
        (status = 400, description = "CSV validation error"),
        (status = 413, description = "Uploaded CSV exceeds 5 MB size limit"),
    ),
    tag = "Blocklist",
    operation_id = "Upload a batch blocklist CSV",
    security(("api_key" = []))
)]
pub async fn upload_batch_blocklist(
    state: web::Data<AppState>,
    req: HttpRequest,
    mut payload: Multipart,
) -> HttpResponse {
    let flow = Flow::BatchBlocklistUpload;
    let mut csv_bytes = bytes::BytesMut::new();
    while let Ok(Some(field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name();
        if matches!(field_name, Some("file") | None) {
            let mut stream = field.into_stream();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(data) => csv_bytes.extend_from_slice(&data),
                    Err(e) => {
                        return api::log_and_return_error_response(error_stack::report!(
                            errors::ApiErrorResponse::InvalidRequestData {
                                message: format!("Failed to read uploaded file: {e}"),
                            }
                        ))
                    }
                }
            }
            break; // only process the first "file" field
        }
    }
    let csv_bytes = csv_bytes.freeze();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _payload, _| {
            let csv_bytes = csv_bytes.clone();
            async move { blocklist::upload_batch_blocklist(state, auth.platform, csv_bytes).await }
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: false,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountWrite,
                allow_connected: false,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[utoipa::path(
    get,
    path = "/blocklist/batch/{job_id}",
    params(
        ("job_id" = String, Path, description = "The job ID returned by the batch upload endpoint"),
    ),
    responses(
        (status = 200, description = "Batch blocklist job status", body = BatchBlocklistJobStatusResponse),
        (status = 404, description = "Job not found"),
    ),
    tag = "Blocklist",
    operation_id = "Get batch blocklist job status",
    security(("api_key" = []))
)]
pub async fn get_batch_blocklist_job_status(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::GetBatchBlocklistJobStatus;
    let job_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        job_id,
        |state, auth: auth::AuthenticationData, job_id, _| {
            blocklist::get_batch_blocklist_job_status(state, auth.platform, job_id)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: false,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountRead,
                allow_connected: false,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[utoipa::path(
    get,
    path = "/blocklist/batch",
    params(
        ("limit" = Option<u32>, Query, description = "Maximum number of jobs to return (default 10)"),
        ("offset" = Option<u32>, Query, description = "Zero-based offset for pagination (default 0)"),
    ),
    responses(
        (status = 200, description = "List of batch blocklist jobs", body = ListBatchBlocklistJobsResponse),
    ),
    tag = "Blocklist",
    operation_id = "List batch blocklist jobs",
    security(("api_key" = []))
)]
pub async fn list_batch_blocklist_jobs(
    state: web::Data<AppState>,
    req: HttpRequest,
    query_payload: web::Query<api_blocklist::ListBatchBlocklistJobsQuery>,
) -> HttpResponse {
    let flow = Flow::ListBatchBlocklistJobs;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        query_payload.into_inner(),
        |state, auth: auth::AuthenticationData, query, _| {
            blocklist::list_batch_blocklist_jobs(state, auth.platform, query)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: false,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountRead,
                allow_connected: false,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
