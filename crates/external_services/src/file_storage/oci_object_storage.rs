//! OCI Object Storage, accessed through its Amazon S3 Compatibility API.

pub use cloud_services::storage::oci::OciObjectStorageConfig as OciFileStorageConfig;

use super::aws_s3::AwsFileStorageClient;

/// Creates a file storage client for OCI Object Storage.
pub(super) async fn client(config: &OciFileStorageConfig) -> AwsFileStorageClient {
    AwsFileStorageClient::new_oci(config).await
}
