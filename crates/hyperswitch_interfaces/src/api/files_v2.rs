//! Files V2 interface

use hyperswitch_domain_models::{
    router_data_v2::FilesFlowData,
    router_flow_types::{Retrieve, Upload},
    router_request_types::{RetrieveFileRequestData, UploadFileRequestData},
    router_response_types::{RetrieveFileResponse, UploadFileResponse},
};

use crate::api::{errors, files::FilePurpose, ConnectorCommon, ConnectorIntegrationV2};

/// trait UploadFileV2
pub trait UploadFileV2:
    ConnectorIntegrationV2<Upload, FilesFlowData, UploadFileRequestData, UploadFileResponse>
{
}

/// trait RetrieveFileV2
pub trait RetrieveFileV2:
    ConnectorIntegrationV2<Retrieve, FilesFlowData, RetrieveFileRequestData, RetrieveFileResponse>
{
}

/// trait FileUploadV2
pub trait FileUploadV2: ConnectorCommon + Sync + UploadFileV2 + RetrieveFileV2 {
    /// fn validate_file_upload_v2
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
