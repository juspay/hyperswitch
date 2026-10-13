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

/// Trait for file storage operations
#[async_trait::async_trait]
pub trait FileStorageInterface: dyn_clone::DynClone + Sync + Send {
    /// Uploads a file to the selected storage scheme.
    async fn upload_file(
        &self,
        file_key: &str,
        file: Vec<u8>,
    ) -> CustomResult<(), FileStorageError>;

    /// Deletes a file from the selected storage scheme.
    async fn delete_file(&self, file_key: &str) -> CustomResult<(), FileStorageError>;

    /// Retrieves a file from the selected storage scheme.
    async fn retrieve_file(&self, file_key: &str) -> CustomResult<Vec<u8>, FileStorageError>;

    /// Begins a multipart upload, returning the identifier every subsequent part must carry.
    async fn create_multipart_upload(
        &self,
        file_key: &str,
    ) -> CustomResult<String, FileStorageError>;

    /// Uploads one part of an in-progress multipart upload, returning that part's entity tag.
    /// Part numbers start at 1. Every part except the last must be at least 5 MB.
    async fn upload_part(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        body: Vec<u8>,
    ) -> CustomResult<String, FileStorageError>;

    /// Assembles the uploaded parts into a single object, returning the backend's expiry
    /// description when a lifecycle rule applies.
    async fn complete_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
        parts: Vec<(i32, String)>,
    ) -> CustomResult<Option<String>, FileStorageError>;

    /// Discards an in-progress multipart upload. Its parts are billed until this is called.
    async fn abort_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
    ) -> CustomResult<(), FileStorageError>;

    /// Returns a time-limited URL granting read access to the file without further authentication.
    async fn get_presigned_download_url(
        &self,
        file_key: &str,
        expires_in: std::time::Duration,
        download_file_name: Option<&str>,
    ) -> CustomResult<url::Url, FileStorageError>;
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

/// Represents errors that can occur during file storage operations.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FileStorageError {
    /// Indicates that the file upload operation failed.
    #[error("Failed to upload file")]
    UploadFailed,

    /// Indicates that the file retrieval operation failed.
    #[error("Failed to retrieve file")]
    RetrieveFailed,

    /// Indicates that the file deletion operation failed.
    #[error("Failed to delete file")]
    DeleteFailed,

    /// Indicates that starting a multipart upload failed.
    #[error("Failed to create multipart upload")]
    MultipartCreateFailed,

    /// Indicates that uploading a part of a multipart upload failed.
    #[error("Failed to upload multipart part")]
    MultipartUploadPartFailed,

    /// Indicates that assembling a multipart upload failed.
    #[error("Failed to complete multipart upload")]
    MultipartCompleteFailed,

    /// Indicates that discarding an in-progress multipart upload failed.
    #[error("Failed to abort multipart upload")]
    MultipartAbortFailed,

    /// Indicates that generating a presigned URL failed.
    #[error("Failed to generate presigned URL")]
    PresignFailed,

    /// Indicates the configured backend cannot issue presigned URLs.
    #[error("Presigned URLs are not supported by the configured file storage backend")]
    PresigningNotSupported,
}
