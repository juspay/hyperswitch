use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use crate::core::api_locking;
pub mod transformers;

use super::app::AppState;
use crate::{
    core::files::*,
    services::{api, authentication as auth},
    types::api::files,
};

/// Files - Create
///
/// To create a file
#[utoipa::path(
    post,
    path = "/files",
    request_body=MultipartRequestWithFile,
    responses(
        (status = 200, description = "File created", body = CreateFileResponse),
        (status = 400, description = "Bad Request")
    ),
    tag = "Files",
    operation_id = "Create a File",
    security(("api_key" = []))
)]
//#\[instrument\(skip_all, fields(flow = ?Flow::CreateFile))]
pub async fn files_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: Multipart,
) -> HttpResponse {
    let flow = Flow::CreateFile;
    let create_file_request_result = transformers::get_create_file_request(payload).await;
    let create_file_request = match create_file_request_result {
        Ok(valid_request) => valid_request,
        Err(err) => return api::log_and_return_error_response(err),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        create_file_request,
        |state, auth, req, _| files_create_core(state, auth.merchant_account, auth.key_store, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::DashboardNoPermissionAuth,
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Files - Delete
///
/// To delete a file
#[utoipa::path(
    delete,
    path = "/files/{file_id}",
    params(
        ("file_id" = String, Path, description = "The identifier for file")
    ),
    responses(
        (status = 200, description = "File deleted"),
        (status = 404, description = "File not found")
    ),
    tag = "Files",
    operation_id = "Delete a File",
    security(("api_key" = []))
)]
//#\[instrument\(skip_all, fields(flow = ?Flow::DeleteFile))]
pub async fn files_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::DeleteFile;
    let file_id = files::FileId {
        file_id: path.into_inner(),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        file_id,
        |state, auth, req, _| files_delete_core(state, auth.merchant_account, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::DashboardNoPermissionAuth,
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Files - Retrieve
///
/// To retrieve a file
#[utoipa::path(
    get,
    path = "/files/{file_id}",
    params(
        ("file_id" = String, Path, description = "The identifier for file")
    ),
    responses(
        (status = 200, description = "File body"),
        (status = 400, description = "Bad Request")
    ),
    tag = "Files",
    operation_id = "Retrieve a File",
    security(("api_key" = []))
)]
//#\[instrument\(skip_all, fields(flow = ?Flow::RetrieveFile))]
pub async fn files_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::RetrieveFile;
    let file_id = files::FileId {
        file_id: path.into_inner(),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        file_id,
        |state, auth, req, _| {
            files_retrieve_core(state, auth.merchant_account, auth.key_store, req)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::DashboardNoPermissionAuth,
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
