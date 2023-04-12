use crate::core::errors;

use super::ConnectorCommon;

#[async_trait::async_trait]
pub trait FileUpload: ConnectorCommon + Sync {
    fn validate_file_upload(
        &self,
        _purpose: api_models::files::FilePurpose,
        _file_size: i64,
        _file_type: Option<mime::Mime>,
    ) -> common_utils::errors::CustomResult<(), errors::ConnectorError> {
        Err(errors::ConnectorError::FileValidationFailed {reason: "".to_owned()}.into())
    }
}
