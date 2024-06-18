pub use hyperswitch_domain_models::router_flow_types::files::{Retrieve, Upload};

use super::ConnectorCommon;
use crate::{core::errors, services, types, types::api::FilePurpose};

pub trait UploadFileV2:
    services::ConnectorIntegrationV2<
    Upload,
    types::FilesFlowData,
    types::UploadFileRequestData,
    types::UploadFileResponse,
>
{
}

pub trait RetrieveFileV2:
    services::ConnectorIntegrationV2<
    Retrieve,
    types::FilesFlowData,
    types::RetrieveFileRequestData,
    types::RetrieveFileResponse,
>
{
}

pub trait FileUploadV2: ConnectorCommon + Sync + UploadFileV2 + RetrieveFileV2 {
    fn validate_file_upload_v2(
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
