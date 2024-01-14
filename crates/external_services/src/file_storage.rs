//!
//! Module for managing file storage operations with support for multiple storage schemes.
//!

use common_utils::{
    errors::{CustomResult, FileStorageError},
    fs_utils,
};
use error_stack::ResultExt;
use storage_impl::errors::ApplicationError;

#[cfg(feature = "aws_s3")]
use crate::file_storage::aws_s3::{AwsFileStorageClient, AwsFileStorageConfig};

/// When the "aws_s3" feature is enabled, the module includes functionality for AWS S3 storage.
#[cfg(feature = "aws_s3")]
pub mod aws_s3;

/// Enum representing different file storage configurations, allowing for multiple storage schemes.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(tag = "file_storage_scheme")]
#[serde(rename_all = "snake_case")]
pub enum FileStorageConfig {
    /// AWS S3 storage configuration.
    #[cfg(feature = "aws_s3")]
    AwsS3 {
        /// Configuration for AWS S3 file storage.
        aws_s3: AwsFileStorageConfig,
    },
    /// Local file system storage configuration.
    #[default]
    FileSystem,
}

impl FileStorageConfig {
    /// Validates the file storage configuration.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        match self {
            #[cfg(feature = "aws_s3")]
            Self::AwsS3 { aws_s3 } => aws_s3.validate(),
            Self::FileSystem => Ok(()),
        }
    }

    /// Retrieves the appropriate file storage client based on the configuration.
    pub async fn get_file_storage_client(&self) -> FileStorageScheme {
        match self {
            #[cfg(feature = "aws_s3")]
            Self::AwsS3 { aws_s3 } => FileStorageScheme::AwsS3 {
                client: aws_s3.get_aws_file_storage_client().await,
            },
            Self::FileSystem => FileStorageScheme::FileSystem(fs_utils::FileSystem),
        }
    }
}

/// Enum representing different file storage clients.
#[derive(Debug, Clone)]
pub enum FileStorageScheme {
    /// AWS S3 file storage client.
    #[cfg(feature = "aws_s3")]
    AwsS3 {
        /// AWS S3 file storage client.
        client: &'static AwsFileStorageClient,
    },
    /// Local file system storage client.
    FileSystem(fs_utils::FileSystem),
}

impl FileStorageScheme {
    /// Uploads a file to the selected storage scheme.
    pub async fn upload_file(
        &self,
        file_key: String,
        file: Vec<u8>,
    ) -> CustomResult<(), FileStorageError> {
        match self {
            #[cfg(feature = "aws_s3")]
            Self::AwsS3 { client } => client
                .upload_file_to_s3(file_key, file)
                .await
                .change_context(FileStorageError::UploadFailed),
            Self::FileSystem(file_system) => file_system
                .save_file_to_fs(file_key, file)
                .change_context(FileStorageError::UploadFailed),
        }
    }

    /// Deletes a file from the selected storage scheme.
    pub async fn delete_file(&self, file_key: String) -> CustomResult<(), FileStorageError> {
        match self {
            #[cfg(feature = "aws_s3")]
            Self::AwsS3 { client } => client
                .delete_file_from_s3(file_key)
                .await
                .change_context(FileStorageError::DeleteFailed),
            Self::FileSystem(file_system) => file_system
                .delete_file_from_fs(file_key)
                .change_context(FileStorageError::DeleteFailed),
        }
    }

    /// Retrieves a file from the selected storage scheme.
    pub async fn retrieve_file(&self, file_key: String) -> CustomResult<Vec<u8>, FileStorageError> {
        match self {
            #[cfg(feature = "aws_s3")]
            Self::AwsS3 { client } => client
                .retrieve_file_from_s3(file_key)
                .await
                .change_context(FileStorageError::RetrieveFailed),
            Self::FileSystem(file_system) => file_system
                .retrieve_file_from_fs(file_key)
                .change_context(FileStorageError::RetrieveFailed),
        }
    }
}
