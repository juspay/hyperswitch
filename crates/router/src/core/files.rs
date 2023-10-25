pub mod helpers;
#[cfg(feature = "s3")]
pub mod s3_utils;

#[cfg(not(feature = "s3"))]
pub mod fs_utils;

use api_models::files;
use error_stack::{IntoReport, ResultExt};

use super::errors::{self, RouterResponse};
use crate::{
    consts,
    routes::AppState,
    services::{self, ApplicationResponse},
    types::{api, domain},
};

pub async fn files_create_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    create_file_request: api::CreateFileRequest,
) -> RouterResponse<files::CreateFileResponse> {
    helpers::validate_file_upload(
        &state,
        merchant_account.clone(),
        create_file_request.clone(),
    )
    .await?;
    let file_id = common_utils::generate_id(consts::ID_LENGTH, "file");
    #[cfg(feature = "s3")]
    let file_key = format!("{}/{}", merchant_account.merchant_id, file_id);
    #[cfg(not(feature = "s3"))]
    let file_key = format!("{}_{}", merchant_account.merchant_id, file_id);
    let file_new = diesel_models::file::FileMetadataNew {
        file_id: file_id.clone(),
        merchant_id: merchant_account.merchant_id.clone(),
        file_name: create_file_request.file_name.clone(),
        file_size: create_file_request.file_size,
        file_type: create_file_request.file_type.to_string(),
        provider_file_id: None,
        file_upload_provider: None,
        available: false,
        connector_label: None,
        profile_id: None,
        merchant_connector_id: None,
    };

    let file_metadata_object = state
        .store
        .insert_file_metadata(file_new)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to insert file_metadata")?;
    let (provider_file_id, file_upload_provider, profile_id, merchant_connector_id) =
        helpers::upload_and_get_provider_provider_file_id_profile_id(
            &state,
            &merchant_account,
            &key_store,
            &create_file_request,
            file_key.clone(),
        )
        .await?;

    // Update file metadata
    let update_file_metadata = diesel_models::file::FileMetadataUpdate::Update {
        provider_file_id: Some(provider_file_id),
        file_upload_provider: Some(file_upload_provider),
        available: true,
        profile_id,
        merchant_connector_id,
    };
    state
        .store
        .as_ref()
        .update_file_metadata(file_metadata_object, update_file_metadata)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Unable to update file_metadata with file_id: {}", file_id)
        })?;
    Ok(services::api::ApplicationResponse::Json(
        files::CreateFileResponse { file_id },
    ))
}

pub async fn files_delete_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    req: api::FileId,
) -> RouterResponse<serde_json::Value> {
    helpers::delete_file_using_file_id(&state, req.file_id.clone(), &merchant_account).await?;
    state
        .store
        .as_ref()
        .delete_file_metadata_by_merchant_id_file_id(&merchant_account.merchant_id, &req.file_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to delete file_metadata")?;
    Ok(ApplicationResponse::StatusOk)
}

pub async fn files_retrieve_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: api::FileId,
) -> RouterResponse<serde_json::Value> {
    let file_metadata_object = state
        .store
        .as_ref()
        .find_file_metadata_by_merchant_id_file_id(&merchant_account.merchant_id, &req.file_id)
        .await
        .change_context(errors::ApiErrorResponse::FileNotFound)
        .attach_printable("Unable to retrieve file_metadata")?;
    let (received_data, _provider_file_id) =
        helpers::retrieve_file_and_provider_file_id_from_file_id(
            &state,
            Some(req.file_id),
            &merchant_account,
            &key_store,
            api::FileDataRequired::Required,
        )
        .await?;
    let content_type = file_metadata_object
        .file_type
        .parse::<mime::Mime>()
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse file content type")?;
    Ok(ApplicationResponse::FileData((
        received_data
            .ok_or(errors::ApiErrorResponse::FileNotAvailable)
            .into_report()
            .attach_printable("File data not found")?,
        content_type,
    )))
}
