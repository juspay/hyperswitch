use actix_multipart::Field;
use common_utils::errors::CustomResult;
use futures::TryStreamExt;

use crate::{
    core::errors,
    routes::AppState,
    types::{api, storage},
};

pub async fn read_string(field: &mut Field) -> Option<String> {
    let bytes = field.try_next().await;
    if let Ok(Some(bytes)) = bytes {
        String::from_utf8(bytes.to_vec()).ok()
    } else {
        None
    }
}

pub async fn get_file_purpose(field: &mut Field) -> Option<api::FilePurpose> {
    let purpose = read_string(field).await;
    match purpose.as_deref() {
        Some("dispute_evidence") => Some(api::FilePurpose::DisputeEvidence),
        _ => None,
    }
}

pub async fn upload_file(
    #[cfg(feature = "s3")] state: &AppState,
    file_key: String,
    file: Vec<u8>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    #[cfg(feature = "s3")]
    return super::s3_utils::upload_file_to_s3(state, file_key, file).await;
    #[cfg(not(feature = "s3"))]
    return super::fs_utils::save_file_to_fs(file_key, file);
}

pub async fn delete_file(
    #[cfg(feature = "s3")] state: &AppState,
    file_key: String,
) -> CustomResult<(), errors::ApiErrorResponse> {
    #[cfg(feature = "s3")]
    return super::s3_utils::delete_file_from_s3(state, file_key).await;
    #[cfg(not(feature = "s3"))]
    return super::fs_utils::delete_file_from_fs(file_key);
}

pub async fn retrieve_file(
    #[cfg(feature = "s3")] state: &AppState,
    file_key: String,
) -> CustomResult<Vec<u8>, errors::ApiErrorResponse> {
    #[cfg(feature = "s3")]
    return super::s3_utils::retrieve_file_from_s3(state, file_key).await;
    #[cfg(not(feature = "s3"))]
    return super::fs_utils::retrieve_file_from_fs(file_key);
}

pub async fn validate_file_upload(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    create_file_request: api::CreateFileRequest,
) -> CustomResult<(), errors::ApiErrorResponse> {
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
                Ok(()) => Ok(()),
                Err(err) => match err.current_context() {
                    errors::ConnectorError::FileValidationFailed { reason } => {
                        Err(errors::ApiErrorResponse::FileValidationFailed {
                            reason: reason.to_string(),
                        }
                        .into())
                    }
                    _ => Err(errors::ApiErrorResponse::InternalServerError.into()),
                },
            }
        }
    }
}
