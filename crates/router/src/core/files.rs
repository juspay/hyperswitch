pub mod transformers;
// use aws_config;
// use aws_sdk_s3::{config::Region, Client};
// use s3_service;
use api_models::files::{self, CreateFileRequest};

use crate::{routes::AppState, types::{storage, api}};

use super::errors::{RouterResponse, self};

pub async fn files_create_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    create_file_request: CreateFileRequest,
) -> RouterResponse<files::CreateFileResponse> {
    let file_data = create_file_request.file.unwrap().concat().to_vec();
    let file_size_bytes = file_data.len();
    print!(">>>create_file_request1 {:?}", create_file_request.file_name.clone());
    print!(">>>create_file_request2 {:?}", create_file_request.file_type.clone());
    print!(">>>create_file_request3 {:?}", file_size_bytes);
    //File Validation based on the purpose of upload
    match files::FilePurpose::DisputeEvidence {
        files::FilePurpose::DisputeEvidence => {
            // let dispute_params = create_file_request.dispute.ok_or(errors::ApiErrorResponse::InvalidRequestData{message: "Dispute Params are missing".to_owned()}).into_report()?;
            let db = &*state.store;
            let dispute = state
                .store
                .find_dispute_by_merchant_id_dispute_id(&merchant_account.merchant_id, &create_file_request.dispute_id.clone().unwrap())
                .await
                .map_err(|error| errors::StorageErrorExt::to_not_found_response(error, errors::ApiErrorResponse::DisputeNotFound { dispute_id: create_file_request.dispute_id.unwrap() }))?;
            let connector_data = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &dispute.connector,
                api::GetToken::Connector,
            )?;
            let validation = connector_data.connector
                .validate_file_upload(create_file_request.purpose, file_size_bytes as i64, create_file_request.file_type);
            match validation {
                Ok(()) => (),
                Err(err) => match err.current_context() {
                    errors::ConnectorError::FileValidationFailed {reason}  => Err(errors::ApiErrorResponse::FileValidationFailed {reason: reason.to_string()})?,
                    _ => Err(errors::ApiErrorResponse::InternalServerError)?,
                }
            }
            }
    }
    // let config = aws_config::load_from_env().await;
    // let client = aws_sdk_s3::Client::new(&config);
    // let file_name = ""; // TODO
    // let key = ""; // TODO
    // let bucket_name = ""; // TODO
    // s3_service::upload_object(&client, &bucket_name, &file_name, &key).await?;

    // let base64file = consts::BASE64_ENGINE.encode(file_data);
    // print!("payloadBase64: {:?}", base64file);

    // let request = transformers::build_file_upload_request(state, data);
    // let response = services::call_connector_api(state, request)
    //     .await
    //     .change_context(errors::ApiErrorResponse::InternalServerError)?;
    // match response {
    //     Ok(res) => {
    //         Err(errors::ApiErrorResponse::Unauthorized.into())
    //     }
    //     Err(err) => Err(errors::ApiErrorResponse::InternalServerError)
    //         .into_report()
    //         .attach_printable(format!("Got 4xx from the file uploader service: {err:?}")),
    // }
    Err(errors::ApiErrorResponse::Unauthorized.into())
}
