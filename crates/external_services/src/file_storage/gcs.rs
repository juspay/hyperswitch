use common_utils::{errors::CustomResult, ext_traits::ConfigExt};
use error_stack::ResultExt;
use google_cloud_storage::client::{Storage, StorageControl};

use super::InvalidFileStorageConfig;
use crate::file_storage::{FileStorageError, FileStorageInterface};

/// Configuration for GCP Cloud Storage file storage.
#[derive(Debug, serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct GcsFileStorageConfig {
    /// The GCS bucket to send file uploads
    bucket_name: String,
}

impl GcsFileStorageConfig {
    /// Validates the GCP Cloud Storage file storage configuration.
    pub(super) fn validate(&self) -> Result<(), InvalidFileStorageConfig> {
        use common_utils::fp_utils::when;

        when(self.bucket_name.is_default_or_empty(), || {
            Err(InvalidFileStorageConfig(
                "gcp storage bucket name must not be empty",
            ))
        })
    }
}

/// GCP Cloud Storage file storage client.
#[derive(Debug, Clone)]
pub(super) struct GcsFileStorageClient {
    /// GCS client for object uploads and downloads.
    data: Storage,
    /// GCS client for object deletes.
    control: StorageControl,
    /// The name of the GCS bucket, pre-formatted as `projects/_/buckets/{bucket_id}`.
    bucket_name: String,
}

impl GcsFileStorageClient {
    /// Creates a new GCP Cloud Storage file storage client.
    pub(super) async fn new(config: &GcsFileStorageConfig) -> CustomResult<Self, GcsError> {
        let data = Storage::builder()
            .build()
            .await
            .change_context(GcsError::ClientCreationFailed)?;
        let control = StorageControl::builder()
            .build()
            .await
            .change_context(GcsError::ClientCreationFailed)?;
        Ok(Self {
            data,
            control,
            bucket_name: format!("projects/_/buckets/{}", config.bucket_name),
        })
    }

    /// Uploads a file to GCS.
    async fn upload_file(&self, file_key: &str, file: Vec<u8>) -> CustomResult<(), GcsError> {
        Box::pin(
            self.data
                .write_object(
                    self.bucket_name.clone(),
                    file_key.to_owned(),
                    bytes::Bytes::from(file),
                )
                .send_unbuffered(),
        )
        .await
        .change_context(GcsError::UploadFailed)?;
        Ok(())
    }

    /// Deletes a file from GCS.
    async fn delete_file(&self, file_key: &str) -> CustomResult<(), GcsError> {
        self.control
            .delete_object()
            .set_bucket(self.bucket_name.clone())
            .set_object(file_key.to_owned())
            .send()
            .await
            .change_context(GcsError::DeleteFailed)?;
        Ok(())
    }

    /// Retrieves a file from GCS.
    async fn retrieve_file(&self, file_key: &str) -> CustomResult<Vec<u8>, GcsError> {
        let mut response = self
            .data
            .read_object(self.bucket_name.clone(), file_key.to_owned())
            .send()
            .await
            .change_context(GcsError::RetrieveFailed)?;
        let mut contents = Vec::new();
        while let Some(chunk) = response
            .next()
            .await
            .transpose()
            .change_context(GcsError::RetrieveFailed)?
        {
            contents.extend_from_slice(&chunk);
        }
        Ok(contents)
    }
}

#[async_trait::async_trait]
impl FileStorageInterface for GcsFileStorageClient {
    /// Uploads a file to GCS.
    async fn upload_file(
        &self,
        file_key: &str,
        file: Vec<u8>,
    ) -> CustomResult<(), FileStorageError> {
        Box::pin(self.upload_file(file_key, file))
            .await
            .change_context(FileStorageError::UploadFailed)?;
        Ok(())
    }

    /// Deletes a file from GCS.
    async fn delete_file(&self, file_key: &str) -> CustomResult<(), FileStorageError> {
        self.delete_file(file_key)
            .await
            .change_context(FileStorageError::DeleteFailed)?;
        Ok(())
    }

    /// Retrieves a file from GCS.
    async fn retrieve_file(&self, file_key: &str) -> CustomResult<Vec<u8>, FileStorageError> {
        Ok(self
            .retrieve_file(file_key)
            .await
            .change_context(FileStorageError::RetrieveFailed)?)
    }
}

/// Enum representing errors that can occur during GCP Cloud Storage file storage operations.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum GcsError {
    /// Error indicating that file upload to GCS failed.
    #[error("File upload to GCS failed")]
    UploadFailed,

    /// Error indicating that file retrieval from GCS failed.
    #[error("File retrieve from GCS failed")]
    RetrieveFailed,

    /// Error indicating that file deletion from GCS failed.
    #[error("File delete from GCS failed")]
    DeleteFailed,

    /// Error indicating that the GCS client could not be created.
    #[error("Failed to create GCS client")]
    ClientCreationFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_fails_when_bucket_name_is_empty() {
        let config = GcsFileStorageConfig {
            bucket_name: String::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_succeeds_when_bucket_name_is_populated() {
        let config = GcsFileStorageConfig {
            bucket_name: "bucket".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    /// Requires a real GCS bucket and `GOOGLE_APPLICATION_CREDENTIALS` to pass.
    #[tokio::test]
    async fn check_gcs_file_round_trip() {
        let config = GcsFileStorageConfig {
            bucket_name: "YOUR GCP STORAGE BUCKET NAME".to_string(),
        };
        let client = GcsFileStorageClient::new(&config)
            .await
            .expect("gcs client creation failed");

        let file_key = "gcs-file-storage-round-trip-test.txt";
        let contents = b"hello".to_vec();

        Box::pin(client.upload_file(file_key, contents.clone()))
            .await
            .expect("gcs file upload failed");

        let retrieved_contents = client
            .retrieve_file(file_key)
            .await
            .expect("gcs file retrieval failed");
        assert_eq!(retrieved_contents, contents);

        client
            .delete_file(file_key)
            .await
            .expect("gcs file deletion failed");
    }
}
