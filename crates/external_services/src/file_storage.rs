//! Module for managing file storage operations with support for multiple storage schemes.

use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

use common_utils::errors::CustomResult;

/// Includes functionality for AWS S3 storage operations.
#[cfg(feature = "aws_s3")]
mod aws_s3;

mod file_system;

/// Enum representing different file storage configurations, allowing for multiple storage schemes.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(tag = "file_storage_backend")]
#[serde(rename_all = "snake_case")]
pub enum FileStorageConfig {
    /// AWS S3 storage configuration.
    #[cfg(feature = "aws_s3")]
    AwsS3 {
        /// Configuration for AWS S3 file storage.
        aws_s3: aws_s3::AwsFileStorageConfig,
    },
    /// Local file system storage configuration.
    #[default]
    FileSystem,
}

impl FileStorageConfig {
    /// Validates the file storage configuration.
    pub fn validate(&self) -> Result<(), InvalidFileStorageConfig> {
        match self {
            #[cfg(feature = "aws_s3")]
            Self::AwsS3 { aws_s3 } => aws_s3.validate(),
            Self::FileSystem => Ok(()),
        }
    }

    /// Retrieves the appropriate file storage client based on the file storage configuration.
    pub async fn get_file_storage_client(&self) -> Arc<dyn FileStorageInterface> {
        match self {
            #[cfg(feature = "aws_s3")]
            Self::AwsS3 { aws_s3 } => Arc::new(aws_s3::AwsFileStorageClient::new(aws_s3).await),
            Self::FileSystem => Arc::new(file_system::FileSystem),
        }
    }
}

#[async_trait::async_trait]
pub trait FileStorageInterface: dyn_clone::DynClone + Sync + Send {
    async fn upload_file(
        &self,
        file_key: &str,
        file: Vec<u8>,
    ) -> CustomResult<(), FileStorageError>;

    async fn delete_file(&self, file_key: &str) -> CustomResult<(), FileStorageError>;

    async fn retrieve_file(&self, file_key: &str) -> CustomResult<Vec<u8>, FileStorageError>;

    async fn initiate_multipart_upload(
        &self,
        file_key: &str,
        content_type: &str,
    ) -> CustomResult<String, FileStorageError> {
        Err(FileStorageError::NotSupported.into())
    }

    async fn upload_part(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        body: aws_sdk_s3::primitives::ByteStream,
    ) -> CustomResult<String, FileStorageError> {
        Err(FileStorageError::NotSupported.into())
    }

    async fn complete_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
        parts: Vec<api_models::revenue_recovery_reports::CompletedPart>,
    ) -> CustomResult<(), FileStorageError> {
        Err(FileStorageError::NotSupported.into())
    }
}

dyn_clone::clone_trait_object!(FileStorageInterface);

/// Error thrown when the file storage config is invalid
#[derive(Debug, Clone)]
pub struct InvalidFileStorageConfig(&'static str);

impl std::error::Error for InvalidFileStorageConfig {}

impl Display for InvalidFileStorageConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "file_storage: {}", self.0)
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FileStorageError {
    #[error("Failed to upload file")]
    UploadFailed,

    #[error("Failed to retrieve file")]
    RetrieveFailed,

    #[error("Failed to delete file")]
    DeleteFailed,

    #[error("Operation not supported by this file storage backend")]
    NotSupported,
}
