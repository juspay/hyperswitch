pub mod transformers;
use api_models::files;
use aws_config::{self, meta::region::RegionProviderChain};
use aws_sdk_s3::{config::Region, Client};
use error_stack::ResultExt;
use futures::TryStreamExt;

use super::errors::{self, RouterResponse};
use crate::{
    consts, logger,
    routes::AppState,
    services::{self, ApplicationResponse},
    types::{api, storage},
};

pub async fn files_create_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    create_file_request: api::CreateFileRequest,
) -> RouterResponse<files::CreateFileResponse> {
    //File Validation based on the purpose of file upload
    match create_file_request.purpose {
        api::FilePurpose::DisputeEvidence => {
            let dispute_id = &create_file_request
                .dispute_id
                .ok_or(errors::ApiErrorResponse::MissingDisputeId)?;
            let dispute = state
                .store
                .find_dispute_by_merchant_id_dispute_id(&merchant_account.merchant_id, dispute_id)
                .await
                .map_err(|error| {
                    errors::StorageErrorExt::to_not_found_response(
                        error,
                        errors::ApiErrorResponse::DisputeNotFound {
                            dispute_id: dispute_id.to_string(),
                        },
                    )
                })?;
            let connector_data = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &dispute.connector,
                api::GetToken::Connector,
            )?;
            let validation = connector_data.connector.validate_file_upload(
                create_file_request.purpose,
                create_file_request.file_size,
                create_file_request.file_type.clone(),
            );
            match validation {
                Ok(()) => (),
                Err(err) => match err.current_context() {
                    errors::ConnectorError::FileValidationFailed { reason } => {
                        Err(errors::ApiErrorResponse::FileValidationFailed {
                            reason: reason.to_string(),
                        })?
                    }
                    _ => Err(errors::ApiErrorResponse::InternalServerError)?,
                },
            }
        }
    }
    let file_id = common_utils::generate_id(consts::ID_LENGTH, "file");
    // Initialize AWS SDK from config
    let region_provider =
        RegionProviderChain::first_try(Region::new(state.conf.file_upload_config.region.clone()));
    let sdk_config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&sdk_config);
    let bucket_name = &state.conf.file_upload_config.bucket_name;
    let file_key = format!("{}_{}", merchant_account.merchant_id, file_id);
    // Upload file to S3
    let upload_res = client
        .put_object()
        .bucket(bucket_name)
        .key(file_key.clone())
        .body(create_file_request.file.into())
        .send()
        .await;
    match upload_res {
        Ok(_) => (),
        Err(error) => {
            logger::error!(?error);
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    }
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
    // Initialize AWS SDK from config
    let region_provider =
        RegionProviderChain::first_try(Region::new(state.conf.file_upload_config.region.clone()));
    let sdk_config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&sdk_config);
    let bucket_name = &state.conf.file_upload_config.bucket_name;
    // Delete file from S3
    let delete_res = client
        .delete_object()
        .bucket(bucket_name)
        .key(file_object.provider_file_id)
        .send()
        .await;
    match delete_res {
        Ok(_) => (),
        Err(error) => {
            logger::error!(?error);
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    }
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
    // Initialize AWS SDK from config
    let region_provider =
        RegionProviderChain::first_try(Region::new(state.conf.file_upload_config.region.clone()));
    let sdk_config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&sdk_config);
    let bucket_name = &state.conf.file_upload_config.bucket_name;
    let mut recieved_data: Vec<u8> = Vec::new();
    // Get file data from S3
    let get_res = client
        .get_object()
        .bucket(bucket_name)
        .key(file_object.provider_file_id)
        .send()
        .await;
    let mut object = match get_res {
        Ok(valid_res) => valid_res,
        Err(error) => {
            logger::error!(?error);
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    };
    while let Some(bytes) = object
        .body
        .try_next()
        .await
        .map_err(|_| errors::ApiErrorResponse::InternalServerError)?
    {
        recieved_data.extend_from_slice(&bytes); // Collect the bytes in the Vec
    }
    let content_type = file_object
        .file_type
        .parse::<mime::Mime>()
        .map_err(|_| errors::ApiErrorResponse::InternalServerError)?;
    Ok(ApplicationResponse::FileData((recieved_data, content_type)))
}
