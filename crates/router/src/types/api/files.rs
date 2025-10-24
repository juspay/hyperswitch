use api_models::enums::FileUploadProvider;
pub use hyperswitch_domain_models::router_flow_types::files::{Retrieve, Upload};
pub use hyperswitch_interfaces::api::files::{FilePurpose, FileUpload, RetrieveFile, UploadFile};
use masking::{Deserialize, Serialize};
use serde_with::serde_as;

pub use super::files_v2::{FileUploadV2, RetrieveFileV2, UploadFileV2};
use crate::{
    core::errors,
    types::{self, transformers::ForeignTryFrom},
};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct FileId {
    pub file_id: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct FileRetrieveRequest {
    pub file_id: String,
    pub dispute_id: Option<String>,
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
            FileUploadProvider::Worldpayvantiv => Ok(Self::Worldpayvantiv),
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
            types::Connector::Worldpayvantiv => Ok(Self::Worldpayvantiv),
            _ => Err(errors::ApiErrorResponse::NotSupported {
                message: "Connector not supported as file provider".to_owned(),
            }
            .into()),
        }
    }
}

#[serde_as]
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateFileRequest {
    pub file: Vec<u8>,
    pub file_name: Option<String>,
    pub file_size: i32,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub file_type: mime::Mime,
    pub purpose: FilePurpose,
    pub dispute_id: Option<String>,
}
