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
use error_stack::{Report, ResultExt};

use super::InvalidFileStorageConfig;
use crate::file_storage::{CompletedPart, FileStorageError, FileStorageInterface};

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
        let response = self
            .inner_client
            .create_multipart_upload()
            .bucket(&self.bucket_name)
            .key(file_key)
            .content_type(content_type)
            .send()
            .await
            .map_err(AwsS3StorageError::CreateMultipartUploadFailure)?;

        response
            .upload_id()
            .ok_or(AwsS3StorageError::MissingUploadId)
            .map_err(Report::from)
            .map(|id| id.to_string())
    }

    async fn upload_part(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        body: Vec<u8>,
    ) -> CustomResult<String, AwsS3StorageError> {
        let response = self
            .inner_client
            .upload_part()
            .bucket(&self.bucket_name)
            .key(file_key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(ByteStream::from(body))
            .send()
            .await
            .map_err(AwsS3StorageError::UploadPartFailure)?;

        response
            .e_tag()
            .ok_or(AwsS3StorageError::MissingETag)
            .map_err(Report::from)
            .map(|etag| etag.to_string())
    }

    async fn complete_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
        parts: Vec<CompletedPart>,
    ) -> CustomResult<(), AwsS3StorageError> {
        let completed_parts: Vec<SdkCompletedPart> = parts
            .into_iter()
            .map(|part| {
                SdkCompletedPart::builder()
                    .part_number(part.part_number)
                    .e_tag(&part.e_tag)
                    .build()
            })
            .collect();

        let completed_multipart = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        self.inner_client
            .complete_multipart_upload()
            .bucket(&self.bucket_name)
            .key(file_key)
            .upload_id(upload_id)
            .multipart_upload(completed_multipart)
            .send()
            .await
            .map_err(AwsS3StorageError::CompleteMultipartUploadFailure)?;

        Ok(())
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
        body: Vec<u8>,
    ) -> CustomResult<String, FileStorageError> {
        self.upload_part(file_key, upload_id, part_number, body)
            .await
            .change_context(FileStorageError::UploadFailed)
    }

    async fn complete_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
        parts: Vec<CompletedPart>,
    ) -> CustomResult<(), FileStorageError> {
        self.complete_multipart_upload(file_key, upload_id, parts)
            .await
            .change_context(FileStorageError::UploadFailed)
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

    /// Unknown error occurred.
    #[error("Unknown error occurred: {0:?}")]
    UnknownError(aws_sdk_s3::primitives::ByteStreamError),

    #[error("Create multipart upload failed: {0:?}")]
    CreateMultipartUploadFailure(aws_sdk_s3::error::SdkError<CreateMultipartUploadError>),

    #[error("Upload ID missing")]
    MissingUploadId,

    #[error("Upload part failed: {0:?}")]
    UploadPartFailure(aws_sdk_s3::error::SdkError<UploadPartError>),

    #[error("ETag missing")]
    MissingETag,

    #[error("Complete multipart upload failed: {0:?}")]
    CompleteMultipartUploadFailure(aws_sdk_s3::error::SdkError<CompleteMultipartUploadError>),
}
