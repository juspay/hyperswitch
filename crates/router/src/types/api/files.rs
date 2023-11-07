use api_models::enums::FileUploadProvider;
use masking::{Deserialize, Serialize};

use super::ConnectorCommon;
use crate::{
    core::errors,
    services,
    types::{self, transformers::ForeignTryFrom},
};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct FileId {
    pub file_id: String,
}

#[derive(Debug)]
pub enum FileDataRequired {
    Required,
    NotRequired,
}

impl ForeignTryFrom<FileUploadProvider> for types::Connector {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(item: FileUploadProvider) -> Result<Self, Self::Error> {
        match item {
            FileUploadProvider::Stripe => Ok(Self::Stripe),
            FileUploadProvider::Checkout => Ok(Self::Checkout),
            FileUploadProvider::Router => Err(errors::ApiErrorResponse::NotSupported {
                message: "File upload provider is not a connector".to_owned(),
            }
            .into()),
        }
    }
}

impl ForeignTryFrom<&types::Connector> for FileUploadProvider {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(item: &types::Connector) -> Result<Self, Self::Error> {
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateFileRequest {
    pub file: Vec<u8>,
    pub file_name: Option<String>,
    pub file_size: i32,
    #[serde(serialize_with = "crate::utils::custom_serde::display_serialize")]
    pub file_type: mime::Mime,
    pub purpose: FilePurpose,
    pub dispute_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, strum::Display, Clone, serde::Serialize)]
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

#[derive(Debug, Clone)]
pub struct Retrieve;

pub trait RetrieveFile:
    services::ConnectorIntegration<
    Retrieve,
    types::RetrieveFileRequestData,
    types::RetrieveFileResponse,
>
{
}

pub trait FileUpload: ConnectorCommon + Sync + UploadFile + RetrieveFile {
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
