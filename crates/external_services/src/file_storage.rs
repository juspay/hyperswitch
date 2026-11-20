//! Module for managing file storage operations with support for multiple storage schemes.

use std::sync::Arc;

use common_utils::errors::CustomResult;

/// Includes functionality for AWS S3 storage operations, and the S3 client that OCI Object
/// Storage and Google Cloud Storage also use.
#[cfg(any(
    feature = "aws_s3",
    feature = "oci_object_storage",
    feature = "gcs_object_storage"
))]
mod aws_s3;

mod file_system;

/// Includes functionality for OCI Object Storage, through its S3 Compatibility API.
#[cfg(feature = "oci_object_storage")]
mod oci_object_storage;

/// Includes functionality for Google Cloud Storage, through its XML API.
#[cfg(feature = "gcs_object_storage")]
mod gcs_object_storage;

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
    /// OCI Object Storage configuration.
    #[cfg(feature = "oci_object_storage")]
    OciObjectStorage {
        /// Configuration for OCI Object Storage.
        oci_object_storage: oci_object_storage::OciFileStorageConfig,
    },
    /// Google Cloud Storage configuration.
    #[cfg(feature = "gcs_object_storage")]
    GcsObjectStorage {
        /// Configuration for Google Cloud Storage.
        gcs_object_storage: gcs_object_storage::GcsFileStorageConfig,
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
            Self::AwsS3 { aws_s3 } => aws_s3.validate()?,
            #[cfg(feature = "oci_object_storage")]
            Self::OciObjectStorage { oci_object_storage } => oci_object_storage.validate()?,
            #[cfg(feature = "gcs_object_storage")]
            Self::GcsObjectStorage { gcs_object_storage } => gcs_object_storage.validate()?,
            Self::FileSystem => {}
        }

        Ok(())
    }

    /// Retrieves the appropriate file storage client based on the file storage configuration.
    pub async fn get_file_storage_client(&self) -> Arc<dyn FileStorageInterface> {
        match self {
            #[cfg(feature = "aws_s3")]
            Self::AwsS3 { aws_s3 } => Arc::new(aws_s3::AwsFileStorageClient::new(aws_s3).await),
            #[cfg(feature = "oci_object_storage")]
            Self::OciObjectStorage { oci_object_storage } => {
                Arc::new(oci_object_storage::client(oci_object_storage).await)
            }
            #[cfg(feature = "gcs_object_storage")]
            Self::GcsObjectStorage { gcs_object_storage } => {
                Arc::new(gcs_object_storage::client(gcs_object_storage).await)
            }
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

/// Writes a file of unknown size as a multipart upload, buffering writes into parts of the 5 MB
/// minimum every part but the last must reach.
#[allow(missing_debug_implementations)]
pub struct MultipartUpload<'a> {
    client: &'a dyn FileStorageInterface,
    file_key: &'a str,
    upload_id: String,
    buffer: Vec<u8>,
    parts: Vec<(i32, String)>,
}

impl<'a> MultipartUpload<'a> {
    /// The minimum size of every part but the last.
    pub const PART_SIZE: usize = 5 * 1024 * 1024;

    /// Begins an upload to `file_key`.
    pub async fn start(
        client: &'a dyn FileStorageInterface,
        file_key: &'a str,
    ) -> CustomResult<Self, FileStorageError> {
        Ok(Self {
            client,
            file_key,
            upload_id: client.create_multipart_upload(file_key).await?,
            buffer: Vec::with_capacity(Self::PART_SIZE),
            parts: Vec::new(),
        })
    }

    /// Appends `bytes`, uploading a part once the buffer reaches `PART_SIZE`.
    pub async fn write(&mut self, bytes: &[u8]) -> CustomResult<(), FileStorageError> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() >= Self::PART_SIZE {
            self.upload_buffer().await?;
        }
        Ok(())
    }

    /// Uploads what remains and assembles the file, returning the backend's expiry description
    /// when a lifecycle rule applies.
    pub async fn finish(mut self) -> CustomResult<Option<String>, FileStorageError> {
        if !self.buffer.is_empty() || self.parts.is_empty() {
            self.upload_buffer().await?;
        }
        self.client
            .complete_multipart_upload(self.file_key, &self.upload_id, self.parts)
            .await
    }

    /// Discards the upload, so its parts stop being billed.
    pub async fn abort(self) -> CustomResult<(), FileStorageError> {
        self.client
            .abort_multipart_upload(self.file_key, &self.upload_id)
            .await
    }

    async fn upload_buffer(&mut self) -> CustomResult<(), FileStorageError> {
        let part_number = i32::try_from(self.parts.len() + 1)
            .map_err(|_| error_stack::report!(FileStorageError::MultipartUploadPartFailed))?;
        let e_tag = self
            .client
            .upload_part(
                self.file_key,
                &self.upload_id,
                part_number,
                std::mem::take(&mut self.buffer),
            )
            .await?;
        self.parts.push((part_number, e_tag));
        Ok(())
    }
}

/// Error thrown when the file storage config is invalid
#[derive(Debug, Clone, thiserror::Error)]
pub enum InvalidFileStorageConfig {
    /// The configuration of a cloud storage backend is invalid.
    #[cfg(any(
        feature = "aws_s3",
        feature = "oci_object_storage",
        feature = "gcs_object_storage"
    ))]
    #[error("file_storage: {0}")]
    CloudStorage(#[from] cloud_services::storage::InvalidStorageConfig),
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
