use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use crate::{core::api_locking, services::authorization::permissions::Permission};
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
#[instrument(skip_all, fields(flow = ?Flow::CreateFile))]
/// Asynchronously handles the creation of files by parsing the multipart request payload, extracting the create file request, and then wrapping the file creation logic in a server wrap. This method requires the Appstate, HttpRequest, and Multipart as input parameters and returns an HttpResponse.
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
        |state, auth, req| files_create_core(state, auth.merchant_account, auth.key_store, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::FileWrite),
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
#[instrument(skip_all, fields(flow = ?Flow::DeleteFile))]
/// Handler for deleting files. This method takes in the state of the application, the HTTP request,
/// and the file path to be deleted. It then creates a flow for deleting a file, obtains the file ID,
/// and calls the server_wrap method to handle the deletion of the file. The method also performs
/// authentication using API key and JWT authentication with the permission to write files. Finally,
/// it awaits the result of the deletion operation and returns the HTTP response.
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
        |state, auth, req| files_delete_core(state, auth.merchant_account, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::FileWrite),
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
#[instrument(skip_all, fields(flow = ?Flow::RetrieveFile))]
/// Asynchronously retrieves a file using the given file path and request information, and returns an HTTP response.
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
        |state, auth, req| files_retrieve_core(state, auth.merchant_account, auth.key_store, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::FileRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
