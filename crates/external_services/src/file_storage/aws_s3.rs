use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{
    operation::{
        abort_multipart_upload::AbortMultipartUploadError,
        complete_multipart_upload::CompleteMultipartUploadError,
        create_multipart_upload::CreateMultipartUploadError, delete_object::DeleteObjectError,
        get_object::GetObjectError, put_object::PutObjectError, upload_part::UploadPartError,
    },
    presigning::{PresigningConfig, PresigningConfigError},
    types::{CompletedMultipartUpload, CompletedPart},
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

    /// Begins a multipart upload on AWS S3.
    async fn create_multipart_upload(
        &self,
        file_key: &str,
    ) -> CustomResult<String, AwsS3StorageError> {
        self.inner_client
            .create_multipart_upload()
            .bucket(&self.bucket_name)
            .key(file_key)
            .content_type("text/csv")
            .send()
            .await
            .map_err(AwsS3StorageError::MultipartCreateFailure)?
            .upload_id
            .ok_or_else(|| error_stack::report!(AwsS3StorageError::MissingUploadId))
    }

    /// Uploads a single part of an in-progress multipart upload to AWS S3.
    async fn upload_part(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        body: Vec<u8>,
    ) -> CustomResult<String, AwsS3StorageError> {
        self.inner_client
            .upload_part()
            .bucket(&self.bucket_name)
            .key(file_key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(body.into())
            .send()
            .await
            .map_err(AwsS3StorageError::MultipartUploadPartFailure)?
            .e_tag
            .ok_or_else(|| error_stack::report!(AwsS3StorageError::MissingPartETag))
    }

    /// Assembles the uploaded parts into a single S3 object, in part number order.
    async fn complete_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
        parts: Vec<(i32, String)>,
    ) -> CustomResult<Option<String>, AwsS3StorageError> {
        let completed_parts = parts
            .into_iter()
            .map(|(part_number, e_tag)| {
                CompletedPart::builder()
                    .part_number(part_number)
                    .e_tag(e_tag)
                    .build()
            })
            .collect();

        Ok(self
            .inner_client
            .complete_multipart_upload()
            .bucket(&self.bucket_name)
            .key(file_key)
            .upload_id(upload_id)
            .multipart_upload(
                CompletedMultipartUpload::builder()
                    .set_parts(Some(completed_parts))
                    .build(),
            )
            .send()
            .await
            .map_err(AwsS3StorageError::MultipartCompleteFailure)?
            .expiration)
    }

    /// Discards an in-progress multipart upload and the parts already uploaded for it.
    async fn abort_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
    ) -> CustomResult<(), AwsS3StorageError> {
        self.inner_client
            .abort_multipart_upload()
            .bucket(&self.bucket_name)
            .key(file_key)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(AwsS3StorageError::MultipartAbortFailure)?;
        Ok(())
    }

    /// Signs a time-limited GET for the object with the client's current credentials.
    async fn get_presigned_download_url(
        &self,
        file_key: &str,
        expires_in: std::time::Duration,
        download_file_name: Option<&str>,
    ) -> CustomResult<url::Url, AwsS3StorageError> {
        let presigning_config = PresigningConfig::expires_in(expires_in)
            .map_err(AwsS3StorageError::PresigningConfigFailure)?;

        let request = self
            .inner_client
            .get_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .response_content_type("text/csv");

        let request = match download_file_name {
            Some(name) => {
                request.response_content_disposition(format!("attachment; filename=\"{name}\""))
            }
            None => request,
        };

        let presigned = request
            .presigned(presigning_config)
            .await
            .map_err(AwsS3StorageError::PresignFailure)?;

        url::Url::parse(presigned.uri()).change_context(AwsS3StorageError::PresignedUrlParseFailure)
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

    /// Begins a multipart upload on AWS S3.
    async fn create_multipart_upload(
        &self,
        file_key: &str,
    ) -> CustomResult<String, FileStorageError> {
        Ok(self
            .create_multipart_upload(file_key)
            .await
            .change_context(FileStorageError::MultipartCreateFailed)?)
    }

    /// Uploads a single part of an in-progress multipart upload to AWS S3.
    async fn upload_part(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        body: Vec<u8>,
    ) -> CustomResult<String, FileStorageError> {
        Ok(self
            .upload_part(file_key, upload_id, part_number, body)
            .await
            .change_context(FileStorageError::MultipartUploadPartFailed)?)
    }

    /// Assembles the uploaded parts into a single S3 object.
    async fn complete_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
        parts: Vec<(i32, String)>,
    ) -> CustomResult<Option<String>, FileStorageError> {
        Ok(self
            .complete_multipart_upload(file_key, upload_id, parts)
            .await
            .change_context(FileStorageError::MultipartCompleteFailed)?)
    }

    /// Discards an in-progress multipart upload on AWS S3.
    async fn abort_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
    ) -> CustomResult<(), FileStorageError> {
        self.abort_multipart_upload(file_key, upload_id)
            .await
            .change_context(FileStorageError::MultipartAbortFailed)?;
        Ok(())
    }

    /// Signs a time-limited GET for an object in AWS S3.
    async fn get_presigned_download_url(
        &self,
        file_key: &str,
        expires_in: std::time::Duration,
        download_file_name: Option<&str>,
    ) -> CustomResult<url::Url, FileStorageError> {
        Ok(self
            .get_presigned_download_url(file_key, expires_in, download_file_name)
            .await
            .change_context(FileStorageError::PresignFailed)?)
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

    /// Error indicating that starting a multipart upload on S3 failed.
    #[error("Multipart upload create on S3 failed: {0:?}")]
    MultipartCreateFailure(aws_sdk_s3::error::SdkError<CreateMultipartUploadError>),

    /// Error indicating that uploading a part to S3 failed.
    #[error("Multipart part upload to S3 failed: {0:?}")]
    MultipartUploadPartFailure(aws_sdk_s3::error::SdkError<UploadPartError>),

    /// Error indicating that assembling a multipart upload on S3 failed.
    #[error("Multipart upload complete on S3 failed: {0:?}")]
    MultipartCompleteFailure(aws_sdk_s3::error::SdkError<CompleteMultipartUploadError>),

    /// Error indicating that discarding a multipart upload on S3 failed.
    #[error("Multipart upload abort on S3 failed: {0:?}")]
    MultipartAbortFailure(aws_sdk_s3::error::SdkError<AbortMultipartUploadError>),

    /// Error indicating S3 accepted the multipart upload but returned no upload id.
    #[error("S3 did not return an upload id for the multipart upload")]
    MissingUploadId,

    /// Error indicating S3 accepted the part but returned no entity tag.
    #[error("S3 did not return an entity tag for the uploaded part")]
    MissingPartETag,

    /// Error indicating the presigning configuration was rejected by the SDK.
    #[error("Presigning configuration was rejected: {0:?}")]
    PresigningConfigFailure(PresigningConfigError),

    /// Error indicating that presigning a GET request failed.
    #[error("Presigning an S3 get request failed: {0:?}")]
    PresignFailure(aws_sdk_s3::error::SdkError<GetObjectError>),

    /// Error indicating the presigned URI could not be parsed as a URL.
    #[error("Presigned URI is not a valid URL")]
    PresignedUrlParseFailure,
}
