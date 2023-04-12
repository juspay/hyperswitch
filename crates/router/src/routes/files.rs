use actix_web::{web, web::Bytes, HttpRequest, HttpResponse};
use api_models::files;
use common_utils::ext_traits::ByteSliceExt;
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing, Flow};
use actix_multipart::{Multipart, Field};
use futures::{TryStreamExt, StreamExt};

use super::app::AppState;
use crate::{
    core::{files::*, errors},
    services::{api, authentication as auth},
};

async fn read_string(field: &mut Field) -> Option<String> {
    let bytes = field.try_next().await;
    if let Ok(Some(bytes)) = bytes {
        String::from_utf8(bytes.to_vec()).ok()
    } else {
        None
    }
}

/// Files - Create
///
/// To create a file
#[utoipa::path(
    post,
    path = "/files",
    request_body=MultipartRequestWithFile,
    responses(
        (status = 200, description = "File created", body = CreateFileResponse),
        (status = 400, description = "Missing Mandatory fields")
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
    let mut purpose: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut file_content: Option<Vec<Bytes>> = None;
    let mut file_size: f64 = 0.0;
    let mut file_type: Option<mime::Mime> = None;
    let mut dispute_id: Option<String> = None;
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name().unwrap();
        file_size = match field.headers().get("Content-Length") {
            Some(size) => size.to_str().unwrap_or("").parse::<f64>().unwrap_or(0.0),
            None => 0.0,
        };
        // Parse the different parameters expected in the multipart request
        match field_name {
            "purpose" => {
                purpose = read_string(&mut field).await;
            }
            "file" => {
                file_type = field.content_type().cloned();
                file_name = content_disposition.get_filename().map(String::from);
                // Need to collect the whole file content here instead of passing the stream of bytes to the 'MultipartRequestWithFile' struct
                // because 'Field' is not 'Send'
                file_content = Some(field.map(|chunk| chunk.unwrap()).collect::<Vec<Bytes>>().await);
            }
            "dispute_id" => {
                dispute_id = read_string(&mut field).await;
                // let bytes_a = field.try_next().await;
                // dispute_params = if let Ok(Some(bytes)) = bytes_a {
                //     Some(serde_json::from_slice(&bytes).unwrap()
                //         // .into_report()
                //         // .change_context(errors::ApiErrorResponse::InvalidRequestData { message: "Dispute Params parsing failed".to_owned() })?
                //         )
                // } else {
                //     None
                // };
            }
            // Ignore other parameters
            _ => ()
        }
    }
    let create_file_request =  files::CreateFileRequest {
        file: file_content,
        file_name,
        file_size,
        file_type,
        purpose: files::FilePurpose::DisputeEvidence,
        dispute_id,
    };
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        create_file_request,
        files_create_core,
        &auth::ApiKeyAuth,
    )
    .await
}
