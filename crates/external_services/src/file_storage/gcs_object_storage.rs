//! Google Cloud Storage, accessed through its XML API's Amazon S3 interoperability.

pub use cloud_services::storage::gcs::GcsObjectStorageConfig as GcsFileStorageConfig;

use super::aws_s3::AwsFileStorageClient;

/// Creates a file storage client for Google Cloud Storage.
pub(super) async fn client(config: &GcsFileStorageConfig) -> AwsFileStorageClient {
    AwsFileStorageClient::new_gcs(config).await
}
