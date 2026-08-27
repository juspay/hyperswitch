use actix_multipart::form::{bytes::Bytes as MultipartBytes, MultipartForm};
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::blocklist as api_blocklist;
use error_stack::report;
use router_env::Flow;

use crate::{
    core::{api_locking, blocklist},
    routes::AppState,
    services::{api, authentication as auth, authorization::permissions::Permission},
};

#[utoipa::path(
    post,
    path = "/blocklist",
    request_body = BlocklistRequest,
    params (
        ("X-Profile-Id" = Option<String>, Header, description = "The business profile to block this \
         entry under. Ignored when authenticating with a JWT, which carries its own profile. If \
         omitted, the merchant's default profile is used; merchants with more than one profile have \
         no default and will receive an error asking for this header."),
    ),
    responses(
        (status = 200, description = "Fingerprint Blocked", body = BlocklistResponse),
        (status = 400, description = "Invalid Data, or no profile could be resolved")
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
            let profile_id = auth.profile.map(|profile| profile.get_id().clone());
            blocklist::add_entry_to_blocklist(state, auth.platform, profile_id, body)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountWrite,
                allow_connected: true,
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
    params (
        ("X-Profile-Id" = Option<String>, Header, description = "The business profile to unblock \
         this entry from. Only entries belonging to that profile, or entries with no profile, are \
         removed - an entry blocked under a different profile is not affected and the request \
         returns 404. Resolution follows the same rules as blocking."),
    ),
    responses(
        (status = 200, description = "Fingerprint Unblocked", body = BlocklistResponse),
        (status = 400, description = "Invalid Data, or no profile could be resolved"),
        (status = 404, description = "No entry for this fingerprint under the resolved profile")
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
            let profile_id = auth.profile.map(|profile| profile.get_id().clone());
            blocklist::remove_entry_from_blocklist(
                state,
                auth.platform.get_processor().clone(),
                profile_id,
                body,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountWrite,
                allow_connected: true,
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
        ("X-Profile-Id" = Option<String>, Header, description = "Restricts the listing to entries \
         belonging to this business profile, plus entries with no profile. When no profile can be \
         resolved - as with publishable-key authentication - all of the merchant's entries are \
         returned, as before."),
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
        allow_connected_scope_operation: true,
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

            // None whenever no profile is resolved - SDK/publishable-key auth, or an API key sent
            // without X-Profile-Id - which keeps the listing merchant-wide
            let profile_id = auth.profile.map(|profile| profile.get_id().clone());
            blocklist::list_blocklist_entries(
                state,
                auth.platform.get_processor().clone(),
                profile_id,
                query,
            )
        },
        auth::auth_type(
            &*auth_type,
            &auth::JWTAuth {
                permission: Permission::MerchantAccountRead,
                allow_connected: true,
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
            blocklist::toggle_blocklist_guard(state, auth.platform.get_processor().clone(), query)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountWrite,
                allow_connected: true,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

// ---- Batch blocklist upload route handlers ----

#[derive(Debug, MultipartForm)]
pub struct BatchBlocklistUploadForm {
    // Also charged against `consts::MULTIPART_MEMORY_LIMIT`, the global in-memory budget for
    // multipart fields. That global must stay >= this limit, or this one is unreachable.
    #[multipart(limit = "5MiB")]
    pub file: MultipartBytes,
}

#[utoipa::path(
    post,
    path = "/blocklist/batch",
    request_body(
        content = String,
        content_type = "multipart/form-data",
        description = "A multipart/form-data request with a `file` field containing a UTF-8 CSV (max 5 MiB). \
            The CSV must have a header row: `type,data,metadata`. \
            `type`: one of `card_bin` (6 digits), `extended_card_bin` (8 digits), `fingerprint`. \
            `metadata`: optional, `key=value` pairs separated by `;` (e.g. `reason=fraud;source=manual`). \
            Maximum 100,000 data rows.",
    ),
    params (
        ("X-Profile-Id" = Option<String>, Header, description = "The business profile every entry \
         in this upload is blocked under. Resolution follows the same rules as blocking a single \
         entry."),
    ),
    responses(
        (status = 202, description = "Batch blocklist job initiated", body = BatchBlocklistUploadResponse),
        (status = 400, description = "CSV validation error, file exceeds 5 MiB limit, or no profile could be resolved"),
    ),
    tag = "Blocklist",
    operation_id = "Upload a batch blocklist CSV",
    security(("api_key" = []))
)]
pub async fn upload_batch_blocklist(
    state: web::Data<AppState>,
    req: HttpRequest,
    MultipartForm(form): MultipartForm<BatchBlocklistUploadForm>,
) -> HttpResponse {
    let flow = Flow::BatchBlocklistUpload;
    let csv_bytes = bytes::Bytes::from(form.file.data.to_vec());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _payload, _| {
            let csv_bytes = csv_bytes.clone();
            let profile_id = auth.profile.map(|profile| profile.get_id().clone());
            async move {
                blocklist::upload_batch_blocklist(state, auth.platform, profile_id, csv_bytes).await
            }
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountWrite,
                allow_connected: true,
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
                allow_connected_scope_operation: true,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountRead,
                allow_connected: true,
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
                allow_connected_scope_operation: true,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountRead,
                allow_connected: true,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
