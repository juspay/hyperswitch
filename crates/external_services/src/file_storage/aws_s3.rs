use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{
    operation::{
        complete_multipart_upload::CompleteMultipartUploadError,
        create_multipart_upload::CreateMultipartUploadError, delete_object::DeleteObjectError,
        get_object::GetObjectError, put_object::PutObjectError, upload_part::UploadPartError,
    },
    primitives::ByteStream,
    types::{CompletedMultipartUpload, CompletedPart as SdkCompletedPart},
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
        Self {
            inner_client: Client::new(&sdk_config),
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

    async fn initiate_multipart_upload(
        &self,
        file_key: &str,
        content_type: &str,
    ) -> CustomResult<String, AwsS3StorageError> {
        let create_multipart_upload_output = self
            .inner_client
            .create_multipart_upload()
            .bucket(&self.bucket_name)
            .key(file_key)
            .content_type(content_type)
            .send()
            .await
            .map_err(AwsS3StorageError::CreateMultipartUploadFailure)?;

        create_multipart_upload_output
            .upload_id()
            .map(ToOwned::to_owned)
            .ok_or(AwsS3StorageError::MissingUploadId.into())
    }

    async fn upload_part(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        body: ByteStream,
    ) -> CustomResult<String, AwsS3StorageError> {
        let upload_part_output = self
            .inner_client
            .upload_part()
            .bucket(&self.bucket_name)
            .key(file_key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(body)
            .send()
            .await
            .map_err(AwsS3StorageError::UploadPartFailure)?;

        upload_part_output
            .e_tag()
            .map(ToOwned::to_owned)
            .ok_or(AwsS3StorageError::MissingETag.into())
    }

    async fn complete_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
        parts: Vec<api_models::revenue_recovery_reports::CompletedPart>,
    ) -> CustomResult<(), AwsS3StorageError> {
        let sdk_parts: Vec<SdkCompletedPart> = parts
            .into_iter()
            .map(|p| {
                SdkCompletedPart::builder()
                    .part_number(p.part_number)
                    .e_tag(p.e_tag)
                    .build()
            })
            .collect();

        let completed_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(sdk_parts))
            .build();

        self.inner_client
            .complete_multipart_upload()
            .bucket(&self.bucket_name)
            .key(file_key)
            .upload_id(upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await
            .map_err(AwsS3StorageError::CompleteMultipartUploadFailure)?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl FileStorageInterface for AwsFileStorageClient {
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

    async fn delete_file(&self, file_key: &str) -> CustomResult<(), FileStorageError> {
        self.delete_file(file_key)
            .await
            .change_context(FileStorageError::DeleteFailed)?;
        Ok(())
    }

    async fn retrieve_file(&self, file_key: &str) -> CustomResult<Vec<u8>, FileStorageError> {
        Ok(self
            .retrieve_file(file_key)
            .await
            .change_context(FileStorageError::RetrieveFailed)?)
    }

    async fn initiate_multipart_upload(
        &self,
        file_key: &str,
        content_type: &str,
    ) -> CustomResult<String, FileStorageError> {
        self.initiate_multipart_upload(file_key, content_type)
            .await
            .change_context(FileStorageError::UploadFailed)
    }

    async fn upload_part(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        body: aws_sdk_s3::primitives::ByteStream,
    ) -> CustomResult<String, FileStorageError> {
        self.upload_part(file_key, upload_id, part_number, body)
            .await
            .change_context(FileStorageError::UploadFailed)
    }

    async fn complete_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
        parts: Vec<api_models::revenue_recovery_reports::CompletedPart>,
    ) -> CustomResult<(), FileStorageError> {
        self.complete_multipart_upload(file_key, upload_id, parts)
            .await
            .change_context(FileStorageError::UploadFailed)
    }
}

#[derive(Debug, thiserror::Error)]
enum AwsS3StorageError {
    #[error("File upload to S3 failed: {0:?}")]
    UploadFailure(aws_sdk_s3::error::SdkError<PutObjectError>),

    #[error("File retrieve from S3 failed: {0:?}")]
    RetrieveFailure(aws_sdk_s3::error::SdkError<GetObjectError>),

    #[error("File delete from S3 failed: {0:?}")]
    DeleteFailure(aws_sdk_s3::error::SdkError<DeleteObjectError>),

    #[error("Unknown error occurred: {0:?}")]
    UnknownError(aws_sdk_s3::primitives::ByteStreamError),

    #[error("Failed to initiate multipart upload to S3: {0:?}")]
    CreateMultipartUploadFailure(aws_sdk_s3::error::SdkError<CreateMultipartUploadError>),

    #[error("Missing Upload ID from S3 response")]
    MissingUploadId,

    #[error("Failed to upload part to S3: {0:?}")]
    UploadPartFailure(aws_sdk_s3::error::SdkError<UploadPartError>),

    #[error("Missing ETag from S3 response for part")]
    MissingETag,

    #[error("Failed to complete multipart upload to S3: {0:?}")]
    CompleteMultipartUploadFailure(aws_sdk_s3::error::SdkError<CompleteMultipartUploadError>),
}
