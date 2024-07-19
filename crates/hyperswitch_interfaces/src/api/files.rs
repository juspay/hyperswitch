//! Files interface

use hyperswitch_domain_models::{
    router_flow_types::files::{Retrieve, Upload},
    router_request_types::{RetrieveFileRequestData, UploadFileRequestData},
    router_response_types::{RetrieveFileResponse, UploadFileResponse},
};

use crate::{
    api::{ConnectorCommon, ConnectorIntegration},
    errors,
};

/// enum FilePurpose
#[derive(Debug, serde::Deserialize, strum::Display, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FilePurpose {
    /// DisputeEvidence
    DisputeEvidence,
}

/// trait UploadFile
pub trait UploadFile:
    ConnectorIntegration<Upload, UploadFileRequestData, UploadFileResponse>
{
}

/// trait RetrieveFile
pub trait RetrieveFile:
    ConnectorIntegration<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>
{
}

/// trait FileUpload
pub trait FileUpload: ConnectorCommon + Sync + UploadFile + RetrieveFile {
    /// fn validate_file_upload
    fn validate_file_upload(
        &self,
        _purpose: FilePurpose,
        _file_size: i32,
        _file_type: mime::Mime,
    ) -> common_utils::errors::CustomResult<(), errors::ConnectorError> {
        Err(errors::ConnectorError::FileValidationFailed {
            reason: "".to_owned(),
        }
        .into())
    }
}
