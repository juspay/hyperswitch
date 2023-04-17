use actix_multipart::Multipart;
use actix_web::{web, web::Bytes, HttpRequest, HttpResponse};
use futures::{StreamExt, TryStreamExt};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{errors, files::*},
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
pub async fn files_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    mut payload: Multipart,
) -> HttpResponse {
    let flow = Flow::CreateFile;

    let mut option_purpose: Option<files::FilePurpose> = None;
    let mut dispute_id: Option<String> = None;

    let mut file_name: Option<String> = None;
    let mut file_content: Option<Vec<Bytes>> = None;
    let mut option_file_type: Option<mime::Mime> = None;

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name();
        // Parse the different parameters expected in the multipart request
        match field_name {
            Some("purpose") => {
                option_purpose = transformers::get_file_purpose(&mut field).await;
            }
            Some("file") => {
                option_file_type = field.content_type().cloned();
                file_name = content_disposition.get_filename().map(String::from);

                //Collect the file content and throw error if something fails
                let mut file_data = Vec::new();
                let mut stream = field.into_stream();
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => file_data.push(bytes),
                        Err(_) => {
                            return api::log_and_return_error_response(
                                errors::ApiErrorResponse::InternalServerError.into(),
                            )
                        }
                    }
                }
                file_content = Some(file_data)
            }
            Some("dispute_id") => {
                dispute_id = transformers::read_string(&mut field).await;
            }
            // Can ignore other params
            _ => (),
        }
    }
    let purpose = match option_purpose {
        Some(valid_purpose) => valid_purpose,
        None => {
            return api::log_and_return_error_response(
                errors::ApiErrorResponse::MissingFilePurpose.into(),
            )
        }
    };
    let file = match file_content {
        Some(valid_file_content) => valid_file_content.concat().to_vec(),
        None => {
            return api::log_and_return_error_response(errors::ApiErrorResponse::MissingFile.into())
        }
    };
    let file_size_result: Result<i32, _> = file.len().try_into();
    let file_size = match file_size_result {
        Ok(valid_file_size) => valid_file_size,
        _ => {
            return api::log_and_return_error_response(
                errors::ApiErrorResponse::InternalServerError.into(),
            )
        }
    };
    // Check if empty file and throw error
    if file_size <= 0 {
        return api::log_and_return_error_response(errors::ApiErrorResponse::MissingFile.into());
    }
    let file_type = match option_file_type {
        Some(valid_file_type) => valid_file_type,
        None => {
            return api::log_and_return_error_response(
                errors::ApiErrorResponse::MissingFileContentType.into(),
            )
        }
    };
    let create_file_request = files::CreateFileRequest {
        file,
        file_name,
        file_size,
        file_type,
        purpose,
        dispute_id,
    };
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        create_file_request,
        files_create_core,
        auth::auth_type(&auth::ApiKeyAuth, &auth::JWTAuth, req.headers()),
    )
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
pub async fn files_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::DeleteFile;
    let file_id = files::FileId {
        file_id: path.into_inner(),
    };
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        file_id,
        files_delete_core,
        auth::auth_type(&auth::ApiKeyAuth, &auth::JWTAuth, req.headers()),
    )
    .await
}

/// Files - Create
///
/// To create a file
#[utoipa::path(
    post,
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
pub async fn files_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::RetrieveFile;
    let file_id = files::FileId {
        file_id: path.into_inner(),
    };
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        file_id,
        files_retrieve_core,
        auth::auth_type(&auth::ApiKeyAuth, &auth::JWTAuth, req.headers()),
    )
    .await
}
