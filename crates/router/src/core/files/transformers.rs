use actix_multipart::Field;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use futures::TryStreamExt;

use crate::{
    core::{
        errors::{self, StorageErrorExt},
        payments, utils,
    },
    routes::AppState,
    services,
    types::{self, api, storage},
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
                .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
                    dispute_id: dispute_id.to_string(),
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

pub async fn delete_file_using_file_id(
    state: &AppState,
    file_key: String,
    merchant_account: &storage_models::merchant_account::MerchantAccount,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let file_metadata_object = state
        .store
        .find_file_metadata_by_merchant_id_file_id(&merchant_account.merchant_id, &file_key)
        .await
        .change_context(errors::ApiErrorResponse::FileNotFound)?;
    match file_metadata_object.file_upload_provider {
        storage_models::enums::FileUploadProvider::Hyperswitch => {
            delete_file(
                #[cfg(feature = "s3")]
                state,
                file_metadata_object.provider_file_id,
            )
            .await
        }
        _ => Err(errors::ApiErrorResponse::NotImplemented {
            message: errors::api_error_response::NotImplementedMessage::Default,
        }
        .into()),
    }
}

pub async fn retrieve_file_and_provider_file_id_from_file_id(
    state: &AppState,
    file_id: Option<String>,
    merchant_account: &storage_models::merchant_account::MerchantAccount,
) -> CustomResult<(Option<Vec<u8>>, Option<String>), errors::ApiErrorResponse> {
    match file_id {
        None => Ok((None, None)),
        Some(file_key) => {
            let file_metadata_object = state
                .store
                .find_file_metadata_by_merchant_id_file_id(&merchant_account.merchant_id, &file_key)
                .await
                .change_context(errors::ApiErrorResponse::FileNotFound)?;
            match file_metadata_object.file_upload_provider {
                storage_models::enums::FileUploadProvider::Hyperswitch => Ok((
                    Some(
                        retrieve_file(
                            #[cfg(feature = "s3")]
                            state,
                            file_metadata_object.provider_file_id.clone(),
                        )
                        .await?,
                    ),
                    Some(file_metadata_object.provider_file_id),
                )),
                //TODO: Handle Retrieve for other providers
                _ => Ok((None, Some(file_metadata_object.provider_file_id))),
            }
        }
    }
}

pub async fn upload_and_get_provider_provider_file_id(
    state: &AppState,
    merchant_account: &storage::merchant_account::MerchantAccount,
    create_file_request: &api::CreateFileRequest,
    file_key: String,
) -> CustomResult<(String, api::FileUploadProvider), errors::ApiErrorResponse> {
    match create_file_request.purpose {
        api::FilePurpose::DisputeEvidence => {
            let dispute_id = create_file_request
                .dispute_id
                .clone()
                .ok_or(errors::ApiErrorResponse::MissingDisputeId)?;
            let dispute = state
                .store
                .find_dispute_by_merchant_id_dispute_id(&merchant_account.merchant_id, &dispute_id)
                .await
                .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound { dispute_id })?;
            let connector_data = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &dispute.connector,
                api::GetToken::Connector,
            )?;
            if connector_data.connector_name.supports_file_storage_module() {
                let payment_intent = state
                    .store
                    .find_payment_intent_by_payment_id_merchant_id(
                        &dispute.payment_id,
                        &merchant_account.merchant_id,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
                let payment_attempt = state
                    .store
                    .find_payment_attempt_by_attempt_id_merchant_id(
                        &dispute.attempt_id,
                        &merchant_account.merchant_id,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::Upload,
                    types::UploadFileRequestData,
                    types::UploadFileResponse,
                > = connector_data.connector.get_connector_integration();
                let router_data = utils::construct_upload_file_router_data(
                    state,
                    &payment_intent,
                    &payment_attempt,
                    merchant_account.clone(),
                    create_file_request,
                    &dispute.connector,
                    file_key,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?;
                let response = services::execute_connector_processing_step(
                    state,
                    connector_integration,
                    &router_data,
                    payments::CallConnectorAction::Trigger,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?;
                let upload_file_response = response.response.map_err(|err| {
                    errors::ApiErrorResponse::ExternalConnectorError {
                        code: err.code,
                        message: err.message,
                        connector: dispute.connector.clone(),
                        status_code: err.status_code,
                        reason: err.reason,
                    }
                })?;
                Ok((
                    upload_file_response.provider_file_id,
                    api::FileUploadProvider::try_from(&connector_data.connector_name)?,
                ))
            } else {
                upload_file(
                    #[cfg(feature = "s3")]
                    state,
                    file_key.clone(),
                    create_file_request.file.clone(),
                )
                .await?;
                Ok((file_key, api::FileUploadProvider::Hyperswitch))
            }
        }
    }
}
