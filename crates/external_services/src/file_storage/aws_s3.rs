use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{
    operation::{
        delete_object::DeleteObjectError, get_object::GetObjectError, put_object::PutObjectError,
    },
    presigning::{PresigningConfig, PresigningConfigError},
    Client,
};
use aws_sdk_sts::config::Region;
use common_utils::{errors::CustomResult, ext_traits::ConfigExt};
use error_stack::ResultExt;

use super::InvalidFileStorageConfig;
use crate::file_storage::{FileStorageError, FileStorageInterface};

/// Configuration for AWS S3 file storage.
#[derive(Debug, serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct AwsFileStorageConfig {
    /// The AWS region to send file uploads
    region: String,
    /// The AWS s3 bucket to send file uploads
    bucket_name: String,
}

impl AwsFileStorageConfig {
    /// Validates the AWS S3 file storage configuration.
    pub(super) fn validate(&self) -> Result<(), InvalidFileStorageConfig> {
        use common_utils::fp_utils::when;

        when(self.region.is_default_or_empty(), || {
            Err(InvalidFileStorageConfig("aws s3 region must not be empty"))
        })?;

        when(self.bucket_name.is_default_or_empty(), || {
            Err(InvalidFileStorageConfig(
                "aws s3 bucket name must not be empty",
            ))
        })
    }
}

/// AWS S3 file storage client.
#[derive(Debug, Clone)]
pub(super) struct AwsFileStorageClient {
    /// AWS S3 client
    inner_client: Client,
    /// The name of the AWS S3 bucket.
    bucket_name: String,
}

impl AwsFileStorageClient {
    /// Creates a new AWS S3 file storage client.
    pub(super) async fn new(config: &AwsFileStorageConfig) -> Self {
        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));
        let sdk_config = aws_config::from_env().region(region_provider).load().await;
        // Custom endpoints (e.g. a local AWS emulator) do not resolve virtual hosted
        // bucket DNS, so path style addressing is required there
        let s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
            .force_path_style(sdk_config.endpoint_url().is_some())
            .build();
        Self {
            inner_client: Client::from_conf(s3_config),
            bucket_name: config.bucket_name.clone(),
        }
    }

    /// Uploads a file to AWS S3.
    async fn upload_file(
        &self,
        file_key: &str,
        file: Vec<u8>,
    ) -> CustomResult<(), AwsS3StorageError> {
        self.inner_client
            .put_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .body(file.into())
            .send()
            .await
            .map_err(AwsS3StorageError::UploadFailure)?;
        Ok(())
    }

    /// Deletes a file from AWS S3.
    async fn delete_file(&self, file_key: &str) -> CustomResult<(), AwsS3StorageError> {
        self.inner_client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .send()
            .await
            .map_err(AwsS3StorageError::DeleteFailure)?;
        Ok(())
    }

    /// Retrieves a file from AWS S3.
    async fn retrieve_file(&self, file_key: &str) -> CustomResult<Vec<u8>, AwsS3StorageError> {
        Ok(self
            .inner_client
            .get_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .send()
            .await
            .map_err(AwsS3StorageError::RetrieveFailure)?
            .body
            .collect()
            .await
            .map_err(AwsS3StorageError::UnknownError)?
            .to_vec())
    }

    /// Generates a presigned URL for downloading a file from AWS S3.
    async fn get_signed_url(
        &self,
        file_key: &str,
        expires_in: std::time::Duration,
    ) -> CustomResult<String, AwsS3StorageError> {
        let presigning_config = PresigningConfig::expires_in(expires_in)
            .map_err(AwsS3StorageError::PresigningConfigFailure)?;
        Ok(self
            .inner_client
            .get_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .presigned(presigning_config)
            .await
            .map_err(AwsS3StorageError::PresigningFailure)?
            .uri()
            .to_string())
    }
}

#[async_trait::async_trait]
impl FileStorageInterface for AwsFileStorageClient {
    /// Uploads a file to AWS S3.
    async fn upload_file(
        &self,
        file_key: &str,
        file: Vec<u8>,
    ) -> CustomResult<(), FileStorageError> {
        self.upload_file(file_key, file)
            .await
            .change_context(FileStorageError::UploadFailed)?;
        Ok(())
    }

    /// Deletes a file from AWS S3.
    async fn delete_file(&self, file_key: &str) -> CustomResult<(), FileStorageError> {
        self.delete_file(file_key)
            .await
            .change_context(FileStorageError::DeleteFailed)?;
        Ok(())
    }

    /// Retrieves a file from AWS S3.
    async fn retrieve_file(&self, file_key: &str) -> CustomResult<Vec<u8>, FileStorageError> {
        Ok(self
            .retrieve_file(file_key)
            .await
            .change_context(FileStorageError::RetrieveFailed)?)
    }

    /// Generates a presigned URL for downloading a file from AWS S3.
    async fn get_signed_url(
        &self,
        file_key: &str,
        expires_in: std::time::Duration,
    ) -> CustomResult<String, FileStorageError> {
        Ok(self
            .get_signed_url(file_key, expires_in)
            .await
            .change_context(FileStorageError::SignedUrlGenerationFailed)?)
    }
}

/// Enum representing errors that can occur during AWS S3 file storage operations.
#[derive(Debug, thiserror::Error)]
enum AwsS3StorageError {
    /// Error indicating that file upload to S3 failed.
    #[error("File upload to S3 failed: {0:?}")]
    UploadFailure(aws_sdk_s3::error::SdkError<PutObjectError>),

    /// Error indicating that file retrieval from S3 failed.
    #[error("File retrieve from S3 failed: {0:?}")]
    RetrieveFailure(aws_sdk_s3::error::SdkError<GetObjectError>),

    /// Error indicating that file deletion from S3 failed.
    #[error("File delete from S3 failed: {0:?}")]
    DeleteFailure(aws_sdk_s3::error::SdkError<DeleteObjectError>),

    /// Error indicating that the presigning configuration could not be built.
    #[error("Failed to build S3 presigning configuration: {0:?}")]
    PresigningConfigFailure(PresigningConfigError),

    /// Error indicating that presigned URL generation failed.
    #[error("Presigned URL generation for S3 failed: {0:?}")]
    PresigningFailure(aws_sdk_s3::error::SdkError<GetObjectError>),

    /// Unknown error occurred.
    #[error("Unknown error occurred: {0:?}")]
    UnknownError(aws_sdk_s3::primitives::ByteStreamError),
}
