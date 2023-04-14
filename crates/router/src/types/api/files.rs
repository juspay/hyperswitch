use masking::{Deserialize, Serialize};

use super::ConnectorCommon;
use crate::core::errors;

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct FileId {
    pub file_id: String,
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

#[async_trait::async_trait]
pub trait FileUpload: ConnectorCommon + Sync {
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
