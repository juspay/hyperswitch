#[cfg(feature = "s3")]
pub mod s3_utils;
pub mod transformers;

#[cfg(not(feature = "s3"))]
pub mod fs_utils;

use api_models::files;
use error_stack::{IntoReport, ResultExt};

use super::errors::{self, RouterResponse};
use crate::{
    consts,
    routes::AppState,
    services::{self, ApplicationResponse},
    types::{api, storage, transformers::ForeignInto},
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
    // Check if file upload should be done to connector / self
    let (provider_file_id, file_upload_provider) =
        transformers::upload_and_get_provider_provider_file_id(
            state,
            &merchant_account,
            &create_file_request,
            file_key.clone(),
        )
        .await?;
    let file_new = storage_models::file::FileNew {
        file_id: file_id.clone(),
        merchant_id: merchant_account.merchant_id,
        file_name: create_file_request.file_name,
        file_size: create_file_request.file_size,
        file_type: create_file_request.file_type.to_string(),
        provider_file_id,
        file_upload_provider: file_upload_provider.foreign_into(),
        available: true,
    };
    state
        .store
        .insert_file_metadata(file_new)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to insert file")?;
    Ok(services::api::ApplicationResponse::Json(
        files::CreateFileResponse { file_id },
    ))
}

pub async fn files_delete_core(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    req: api::FileId,
) -> RouterResponse<serde_json::Value> {
    transformers::delete_file_using_file_id(state, req.file_id.clone(), &merchant_account).await?;
    state
        .store
        .delete_file_metadata_by_merchant_id_file_id(&merchant_account.merchant_id, &req.file_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to delete file")?;
    Ok(ApplicationResponse::StatusOk)
}

pub async fn files_retrieve_core(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    req: api::FileId,
) -> RouterResponse<serde_json::Value> {
    let file_metadata_object = state
        .store
        .find_file_metadata_by_merchant_id_file_id(&merchant_account.merchant_id, &req.file_id)
        .await
        .change_context(errors::ApiErrorResponse::FileNotFound)?;
    let (received_data, _provider_file_id) =
        transformers::retrieve_file_and_provider_file_id_from_file_id(
            state,
            Some(req.file_id),
            &merchant_account,
        )
        .await?;
    let content_type = file_metadata_object
        .file_type
        .parse::<mime::Mime>()
        .map_err(|_| errors::ApiErrorResponse::InternalServerError)
        .into_report()
        .attach_printable("Failed to parse file content type")?;
    Ok(ApplicationResponse::FileData((
        received_data.ok_or(errors::ApiErrorResponse::FileNotFound)?,
        content_type,
    )))
}
