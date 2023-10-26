use actix_multipart::Field;
use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use futures::TryStreamExt;

use crate::{
    core::{
        errors::{self, StorageErrorExt},
        files, payments, utils,
    },
    routes::AppState,
    services,
    types::{self, api, domain, transformers::ForeignTryFrom},
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
    return files::s3_utils::upload_file_to_s3(state, file_key, file).await;
    #[cfg(not(feature = "s3"))]
    return files::fs_utils::save_file_to_fs(file_key, file);
}

pub async fn delete_file(
    #[cfg(feature = "s3")] state: &AppState,
    file_key: String,
) -> CustomResult<(), errors::ApiErrorResponse> {
    #[cfg(feature = "s3")]
    return files::s3_utils::delete_file_from_s3(state, file_key).await;
    #[cfg(not(feature = "s3"))]
    return files::fs_utils::delete_file_from_fs(file_key);
}

pub async fn retrieve_file(
    #[cfg(feature = "s3")] state: &AppState,
    file_key: String,
) -> CustomResult<Vec<u8>, errors::ApiErrorResponse> {
    #[cfg(feature = "s3")]
    return files::s3_utils::retrieve_file_from_s3(state, file_key).await;
    #[cfg(not(feature = "s3"))]
    return files::fs_utils::retrieve_file_from_fs(file_key);
}

pub async fn validate_file_upload(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
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
            // Connector is not called for validating the file, connector_id can be passed as None safely
            let connector_data = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &dispute.connector,
                api::GetToken::Connector,
                None,
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
                    //We are using parent error and ignoring this
                    _error => Err(err
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("File validation failed"))?,
                },
            }
        }
    }
}

pub async fn delete_file_using_file_id(
    state: &AppState,
    file_key: String,
    merchant_account: &domain::MerchantAccount,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let file_metadata_object = state
        .store
        .find_file_metadata_by_merchant_id_file_id(&merchant_account.merchant_id, &file_key)
        .await
        .change_context(errors::ApiErrorResponse::FileNotFound)?;
    let (provider, provider_file_id) = match (
        file_metadata_object.file_upload_provider,
        file_metadata_object.provider_file_id,
        file_metadata_object.available,
    ) {
        (Some(provider), Some(provider_file_id), true) => (provider, provider_file_id),
        _ => Err(errors::ApiErrorResponse::FileNotAvailable)
            .into_report()
            .attach_printable("File not available")?,
    };
    match provider {
        diesel_models::enums::FileUploadProvider::Router => {
            delete_file(
                #[cfg(feature = "s3")]
                state,
                provider_file_id,
            )
            .await
        }
        _ => Err(errors::ApiErrorResponse::FileProviderNotSupported {
            message: "Not Supported because provider is not Router".to_string(),
        }
        .into()),
    }
}

pub async fn retrieve_file_from_connector(
    state: &AppState,
    file_metadata: diesel_models::file::FileMetadata,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<Vec<u8>, errors::ApiErrorResponse> {
    let connector = &types::Connector::foreign_try_from(
        file_metadata
            .file_upload_provider
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable("Missing file upload provider")?,
    )?
    .to_string();
    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        connector,
        api::GetToken::Connector,
        file_metadata.merchant_connector_id.clone(),
    )?;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::Retrieve,
        types::RetrieveFileRequestData,
        types::RetrieveFileResponse,
    > = connector_data.connector.get_connector_integration();
    let router_data = utils::construct_retrieve_file_router_data(
        state,
        merchant_account,
        key_store,
        &file_metadata,
        connector,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed constructing the retrieve file router data")?;
    let response = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed while calling retrieve file connector api")?;
    let retrieve_file_response =
        response
            .response
            .map_err(|err| errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: connector.to_string(),
                status_code: err.status_code,
                reason: err.reason,
            })?;
    Ok(retrieve_file_response.file_data)
}

pub async fn retrieve_file_and_provider_file_id_from_file_id(
    state: &AppState,
    file_id: Option<String>,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    is_connector_file_data_required: api::FileDataRequired,
) -> CustomResult<(Option<Vec<u8>>, Option<String>), errors::ApiErrorResponse> {
    match file_id {
        None => Ok((None, None)),
        Some(file_key) => {
            let file_metadata_object = state
                .store
                .find_file_metadata_by_merchant_id_file_id(&merchant_account.merchant_id, &file_key)
                .await
                .change_context(errors::ApiErrorResponse::FileNotFound)?;
            let (provider, provider_file_id) = match (
                file_metadata_object.file_upload_provider,
                file_metadata_object.provider_file_id.clone(),
                file_metadata_object.available,
            ) {
                (Some(provider), Some(provider_file_id), true) => (provider, provider_file_id),
                _ => Err(errors::ApiErrorResponse::FileNotAvailable)
                    .into_report()
                    .attach_printable("File not available")?,
            };
            match provider {
                diesel_models::enums::FileUploadProvider::Router => Ok((
                    Some(
                        retrieve_file(
                            #[cfg(feature = "s3")]
                            state,
                            provider_file_id.clone(),
                        )
                        .await?,
                    ),
                    Some(provider_file_id),
                )),
                _ => {
                    let connector_file_data = match is_connector_file_data_required {
                        api::FileDataRequired::Required => Some(
                            retrieve_file_from_connector(
                                state,
                                file_metadata_object,
                                merchant_account,
                                key_store,
                            )
                            .await?,
                        ),
                        api::FileDataRequired::NotRequired => None,
                    };
                    Ok((connector_file_data, Some(provider_file_id)))
                }
            }
        }
    }
}

//Upload file to connector if it supports / store it in S3 and return file_upload_provider, provider_file_id accordingly
pub async fn upload_and_get_provider_provider_file_id_profile_id(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    create_file_request: &api::CreateFileRequest,
    file_key: String,
) -> CustomResult<
    (
        String,
        api_models::enums::FileUploadProvider,
        Option<String>,
        Option<String>,
    ),
    errors::ApiErrorResponse,
> {
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
                dispute.merchant_connector_id.clone(),
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
                    merchant_account,
                    key_store,
                    create_file_request,
                    &dispute.connector,
                    file_key,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed constructing the upload file router data")?;
                let response = services::execute_connector_processing_step(
                    state,
                    connector_integration,
                    &router_data,
                    payments::CallConnectorAction::Trigger,
                    None,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while calling upload file connector api")?;

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
                    api_models::enums::FileUploadProvider::foreign_try_from(
                        &connector_data.connector_name,
                    )?,
                    payment_intent.profile_id,
                    payment_attempt.merchant_connector_id,
                ))
            } else {
                upload_file(
                    #[cfg(feature = "s3")]
                    state,
                    file_key.clone(),
                    create_file_request.file.clone(),
                )
                .await?;
                Ok((
                    file_key,
                    api_models::enums::FileUploadProvider::Router,
                    None,
                    None,
                ))
            }
        }
    }
}
