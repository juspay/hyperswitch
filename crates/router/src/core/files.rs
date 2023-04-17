#[cfg(feature = "s3")]
pub mod s3_utils;
pub mod transformers;

#[cfg(not(feature = "s3"))]
pub mod fs_utils;

use api_models::files;
use error_stack::ResultExt;

use super::errors::{self, RouterResponse};
use crate::{
    consts,
    routes::AppState,
    services::{self, ApplicationResponse},
    types::{api, storage},
};

pub async fn files_create_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    create_file_request: api::CreateFileRequest,
) -> RouterResponse<files::CreateFileResponse> {
    transformers::validate_file_upload(
        state,
        merchant_account.clone(),
        create_file_request.clone(),
    )
    .await?;
    let file_id = common_utils::generate_id(consts::ID_LENGTH, "file");
    let file_key = format!("{}_{}", merchant_account.merchant_id, file_id);
    transformers::upload_file(
        #[cfg(feature = "s3")]
        state,
        file_key.clone(),
        create_file_request.file,
    )
    .await?;
    let file_new = storage_models::file::FileNew {
        file_id: file_id.clone(),
        merchant_id: merchant_account.merchant_id,
        file_name: create_file_request.file_name,
        file_size: create_file_request.file_size,
        file_type: create_file_request.file_type.to_string(),
        provider_file_id: file_key,
        available: true,
    };
    state
        .store
        .insert_file(file_new)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    Ok(services::api::ApplicationResponse::Json(
        files::CreateFileResponse { file_id },
    ))
}

pub async fn files_delete_core(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    req: api::FileId,
) -> RouterResponse<serde_json::Value> {
    let file_object = state
        .store
        .find_file_by_merchant_id_file_id(&merchant_account.merchant_id, &req.file_id)
        .await
        .change_context(errors::ApiErrorResponse::FileNotFound)?;
    transformers::delete_file(
        #[cfg(feature = "s3")]
        state,
        file_object.provider_file_id,
    )
    .await?;
    state
        .store
        .delete_file_by_merchant_id_file_id(&merchant_account.merchant_id, &req.file_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    Ok(ApplicationResponse::StatusOk)
}

pub async fn files_retrieve_core(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    req: api::FileId,
) -> RouterResponse<serde_json::Value> {
    let file_object = state
        .store
        .find_file_by_merchant_id_file_id(&merchant_account.merchant_id, &req.file_id)
        .await
        .change_context(errors::ApiErrorResponse::FileNotFound)?;
    let recieved_data = transformers::retrieve_file(
        #[cfg(feature = "s3")]
        state,
        file_object.provider_file_id,
    )
    .await?;
    let content_type = file_object
        .file_type
        .parse::<mime::Mime>()
        .map_err(|_| errors::ApiErrorResponse::InternalServerError)?;
    Ok(ApplicationResponse::FileData((recieved_data, content_type)))
}
