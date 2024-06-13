pub use hyperswitch_domain_models::router_flow_types::files::{Retrieve, Upload};

use super::ConnectorCommon;
use crate::{core::errors, services, types, types::api::FilePurpose};

pub trait UploadFileNew:
    services::ConnectorIntegrationNew<
    Upload,
    types::FilesFlowData,
    types::UploadFileRequestData,
    types::UploadFileResponse,
>
{
}

pub trait RetrieveFileNew:
    services::ConnectorIntegrationNew<
    Retrieve,
    types::FilesFlowData,
    types::RetrieveFileRequestData,
    types::RetrieveFileResponse,
>
{
}

pub trait FileUploadNew: ConnectorCommon + Sync + UploadFileNew + RetrieveFileNew {
    fn validate_file_upload_new(
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
