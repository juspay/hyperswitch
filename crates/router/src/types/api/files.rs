use masking::{Deserialize, Serialize};

use super::ConnectorCommon;
use crate::{core::errors, services, types};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct FileId {
    pub file_id: String,
}

#[derive(Debug, Clone, frunk::LabelledGeneric)]
pub enum FileUploadProvider {
    Router,
    Stripe,
    Checkout,
}

impl TryFrom<&types::Connector> for FileUploadProvider {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(item: &types::Connector) -> Result<Self, Self::Error> {
        match *item {
            types::Connector::Stripe => Ok(Self::Stripe),
            types::Connector::Checkout => Ok(Self::Checkout),
            _ => Err(errors::ApiErrorResponse::NotSupported {
                message: "Connector not supported as file provider".to_owned(),
            }
            .into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateFileRequest {
    pub file: Vec<u8>,
    pub file_name: Option<String>,
    pub file_size: i32,
    pub file_type: mime::Mime,
    pub purpose: FilePurpose,
    pub dispute_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, strum::Display, Clone)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FilePurpose {
    DisputeEvidence,
}

#[derive(Debug, Clone)]
pub struct Upload;

pub trait UploadFile:
    services::ConnectorIntegration<Upload, types::UploadFileRequestData, types::UploadFileResponse>
{
}

pub trait FileUpload: ConnectorCommon + Sync + UploadFile {
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
