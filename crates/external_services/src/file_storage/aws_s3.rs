//! Hyperswitch adapter over the shared [`cloud_services`] S3 client.

use cloud_services::storage::s3::S3Storage;
pub use cloud_services::storage::s3::S3StorageConfig as AwsFileStorageConfig;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;

use crate::file_storage::{FileStorageError, FileStorageInterface};

/// AWS S3 file storage client.
#[derive(Debug, Clone)]
pub(super) struct AwsFileStorageClient {
    /// Shared S3 client
    inner: S3Storage,
}

impl AwsFileStorageClient {
    /// Creates a new AWS S3 file storage client.
    pub(super) async fn new(config: &AwsFileStorageConfig) -> Self {
        Self {
            inner: S3Storage::new(config).await,
        }
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
        self.inner
            .upload_file(file_key, file)
            .await
            .change_context(FileStorageError::UploadFailed)
    }

    /// Deletes a file from AWS S3.
    async fn delete_file(&self, file_key: &str) -> CustomResult<(), FileStorageError> {
        self.inner
            .delete_file(file_key)
            .await
            .change_context(FileStorageError::DeleteFailed)
    }

    /// Retrieves a file from AWS S3.
    async fn retrieve_file(&self, file_key: &str) -> CustomResult<Vec<u8>, FileStorageError> {
        self.inner
            .retrieve_file(file_key)
            .await
            .change_context(FileStorageError::RetrieveFailed)
    }

    /// Begins a multipart upload on AWS S3.
    async fn create_multipart_upload(
        &self,
        file_key: &str,
    ) -> CustomResult<String, FileStorageError> {
        self.inner
            .create_multipart_upload(file_key)
            .await
            .change_context(FileStorageError::MultipartCreateFailed)
    }

    /// Uploads a single part of an in-progress multipart upload to AWS S3.
    async fn upload_part(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        body: Vec<u8>,
    ) -> CustomResult<String, FileStorageError> {
        self.inner
            .upload_part(file_key, upload_id, part_number, body)
            .await
            .change_context(FileStorageError::MultipartUploadPartFailed)
    }

    /// Assembles the uploaded parts into a single S3 object.
    async fn complete_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
        parts: Vec<(i32, String)>,
    ) -> CustomResult<Option<String>, FileStorageError> {
        self.inner
            .complete_multipart_upload(file_key, upload_id, parts)
            .await
            .change_context(FileStorageError::MultipartCompleteFailed)
    }

    /// Discards an in-progress multipart upload on AWS S3.
    async fn abort_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
    ) -> CustomResult<(), FileStorageError> {
        self.inner
            .abort_multipart_upload(file_key, upload_id)
            .await
            .change_context(FileStorageError::MultipartAbortFailed)
    }

    /// Signs a time-limited GET for an object in AWS S3.
    async fn get_presigned_download_url(
        &self,
        file_key: &str,
        expires_in: std::time::Duration,
        download_file_name: Option<&str>,
    ) -> CustomResult<url::Url, FileStorageError> {
        self.inner
            .get_presigned_download_url(file_key, expires_in, download_file_name)
            .await
            .change_context(FileStorageError::PresignFailed)
    }
}

#[cfg(test)]
mod tests {
    use crate::file_storage::FileStorageConfig;

    fn file_storage_config(value: serde_json::Value) -> FileStorageConfig {
        #[allow(clippy::expect_used)]
        serde_json::from_value(value).expect("file storage config should deserialize")
    }

    #[test]
    fn aws_s3_backend_config_deserializes_and_validates() {
        let config = file_storage_config(serde_json::json!({
            "file_storage_backend": "aws_s3",
            "aws_s3": {
                "region": "us-east-1",
                "bucket_name": "bucket",
            },
        }));

        assert!(matches!(config, FileStorageConfig::AwsS3 { .. }));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn aws_s3_backend_reports_invalid_config() {
        for (aws_s3, message) in [
            (
                serde_json::json!({ "bucket_name": "bucket" }),
                "file_storage: aws s3 region must not be empty",
            ),
            (
                serde_json::json!({ "region": "us-east-1", "bucket_name": "  " }),
                "file_storage: aws s3 bucket name must not be empty",
            ),
            (
                serde_json::json!({
                    "region": "us-east-1",
                    "bucket_name": "bucket",
                    "endpoint_url": "objectstorage.example.com",
                }),
                "file_storage: aws s3 endpoint url must be an absolute http(s) url",
            ),
        ] {
            let config = file_storage_config(serde_json::json!({
                "file_storage_backend": "aws_s3",
                "aws_s3": aws_s3,
            }));

            assert_eq!(
                config.validate().map_err(|error| error.to_string()),
                Err(message.to_owned())
            );
        }
    }
}
