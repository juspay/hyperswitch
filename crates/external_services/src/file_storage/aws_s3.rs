use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{
    operation::{
        delete_object::DeleteObjectError, get_object::GetObjectError, put_object::PutObjectError,
    },
    Client,
};
use aws_sdk_sts::config::Region;
use futures::TryStreamExt;

use common_utils::{errors::CustomResult, ext_traits::ConfigExt};
use error_stack::{IntoReport, ResultExt};
use storage_impl::errors::ApplicationError;

static AWS_FILE_STORAGE_CLIENT: tokio::sync::OnceCell<AwsFileStorageClient> =
    tokio::sync::OnceCell::const_new();

/// Configuration for AWS S3 file storage.
#[cfg(feature = "aws_s3")]
#[derive(Debug, serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct AwsFileStorageConfig {
    /// The AWS region to send file uploads
    pub region: String,
    /// The AWS s3 bucket to send file uploads
    pub bucket_name: String,
}

impl AwsFileStorageConfig {
    /// Retrieves the AWS S3 file storage client, initializing it if necessary.
    #[inline]
    pub async fn get_aws_file_storage_client(&self) -> &'static AwsFileStorageClient {
        AWS_FILE_STORAGE_CLIENT
            .get_or_init(|| AwsFileStorageClient::new(self))
            .await
    }
}

#[cfg(feature = "aws_s3")]
impl AwsFileStorageConfig {
    /// Validates the AWS S3 file storage configuration.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.region.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "aws s3 region must not be empty".into(),
            ))
        })?;

        when(self.bucket_name.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "aws s3 bucket name must not be empty".into(),
            ))
        })
    }
}

/// AWS S3 file storage client.
#[derive(Debug, Clone)]
pub struct AwsFileStorageClient {
    /// AWS S3 client
    pub inner_client: Client,
    /// The name of the AWS S3 bucket.
    pub bucket_name: String,
}

impl AwsFileStorageClient {
    /// Creates a new AWS S3 file storage client.
    pub async fn new(config: &AwsFileStorageConfig) -> Self {
        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));
        let sdk_config = aws_config::from_env().region(region_provider).load().await;
        println!("config_check {:?}", config);
        Self {
            inner_client: Client::new(&sdk_config),
            bucket_name: config.bucket_name.clone(),
        }
    }

    /// Uploads a file to AWS S3.
    pub async fn upload_file_to_s3(
        &self,
        file_key: String,
        file: Vec<u8>,
    ) -> CustomResult<(), AwsS3StorageError> {
        let bucket_name = self.bucket_name.clone();
        // Upload file to S3
        let upload_res = self
            .inner_client
            .put_object()
            .bucket(bucket_name)
            .key(file_key.clone())
            .body(file.into())
            .send()
            .await;
        upload_res.map_err(AwsS3StorageError::UploadFailure)?;
        Ok(())
    }

    /// Deletes a file from AWS S3.
    pub async fn delete_file_from_s3(
        &self,
        file_key: String,
    ) -> CustomResult<(), AwsS3StorageError> {
        let bucket_name = self.bucket_name.clone();
        // Delete file from S3
        let delete_res = self
            .inner_client
            .delete_object()
            .bucket(bucket_name)
            .key(file_key)
            .send()
            .await;
        delete_res.map_err(AwsS3StorageError::DeleteFailure)?;
        Ok(())
    }

    /// Retrieves a file from AWS S3.
    pub async fn retrieve_file_from_s3(
        &self,
        file_key: String,
    ) -> CustomResult<Vec<u8>, AwsS3StorageError> {
        let bucket_name = self.bucket_name.clone();
        // Get file data from S3
        let get_res = self
            .inner_client
            .get_object()
            .bucket(bucket_name)
            .key(file_key)
            .send()
            .await;
        let mut object = get_res.map_err(AwsS3StorageError::RetrieveFailure)?;
        let mut received_data: Vec<u8> = Vec::new();
        while let Some(bytes) = object
            .body
            .try_next()
            .await
            .into_report()
            .change_context(AwsS3StorageError::InvalidFileRetrieved)?
        {
            received_data.extend_from_slice(&bytes); // Collect the bytes in the Vec
        }
        Ok(received_data)
    }
}

/// Enum representing errors that can occur during AWS S3 file storage operations.
#[derive(Debug, thiserror::Error)]
pub enum AwsS3StorageError {
    /// Error indicating that file upload to S3 failed.
    #[error("File upload to S3 failed {0:?}")]
    UploadFailure(aws_smithy_client::SdkError<PutObjectError>),

    /// Error indicating that file retrieval from S3 failed.
    #[error("File retrieve from S3 failed {0:?}")]
    RetrieveFailure(aws_smithy_client::SdkError<GetObjectError>),

    /// Error indicating that file deletion from S3 failed.
    #[error("File delete from S3 failed {0:?}")]
    DeleteFailure(aws_smithy_client::SdkError<DeleteObjectError>),

    /// Error indicating that invalid file data was received from S3.
    #[error("Invalid file data received from S3")]
    InvalidFileRetrieved,
}
