//! OCI Object Storage, accessed through its Amazon S3 Compatibility API.

use common_utils::{ext_traits::ConfigExt, fp_utils::when};

use super::{
    aws_s3::{AwsFileStorageClient, S3Endpoint},
    InvalidFileStorageConfig,
};

/// Configuration for OCI Object Storage.
///
/// Requests go to the S3 Compatibility API endpoint of the namespace and region, with the bucket
/// in the path, since OCI's TLS certificate does not cover bucket subdomains. Authenticate with an
/// OCI Customer Secret Key, through `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`.
#[derive(Debug, serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct OciFileStorageConfig {
    /// The Object Storage namespace of the tenancy
    namespace: String,
    /// The OCI region identifier of the bucket, such as `us-ashburn-1`
    region: String,
    /// The bucket to send file uploads
    bucket_name: String,
}

impl OciFileStorageConfig {
    /// Validates the OCI Object Storage configuration.
    pub(super) fn validate(&self) -> Result<(), InvalidFileStorageConfig> {
        when(!is_hostname_label(&self.namespace), || {
            Err(InvalidFileStorageConfig(
                "oci object storage namespace must be non-empty and contain only letters, digits and hyphens",
            ))
        })?;

        when(!is_hostname_label(&self.region), || {
            Err(InvalidFileStorageConfig(
                "oci object storage region must be non-empty and contain only letters, digits and hyphens",
            ))
        })?;

        when(self.bucket_name.is_default_or_empty(), || {
            Err(InvalidFileStorageConfig(
                "oci object storage bucket name must not be empty",
            ))
        })
    }

    /// The S3 Compatibility API endpoint of the namespace and region, addressed path-style.
    fn endpoint(&self) -> S3Endpoint {
        S3Endpoint {
            url: Some(format!(
                "https://{}.compat.objectstorage.{}.oraclecloud.com",
                self.namespace, self.region
            )),
            force_path_style: true,
        }
    }
}

/// Whether `value` can be used as one label of the endpoint host name.
fn is_hostname_label(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

/// Creates a file storage client for OCI Object Storage.
pub(super) async fn client(config: &OciFileStorageConfig) -> AwsFileStorageClient {
    AwsFileStorageClient::with_endpoint(&config.region, &config.bucket_name, &config.endpoint())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_storage::{aws_s3::tests::presigned_url, FileStorageConfig};

    fn config(namespace: &str, region: &str, bucket_name: &str) -> OciFileStorageConfig {
        OciFileStorageConfig {
            namespace: namespace.to_owned(),
            region: region.to_owned(),
            bucket_name: bucket_name.to_owned(),
        }
    }

    #[test]
    fn deserializes_as_its_own_backend() {
        #[allow(clippy::expect_used)]
        let file_storage: FileStorageConfig = serde_json::from_value(serde_json::json!({
            "file_storage_backend": "oci_object_storage",
            "oci_object_storage": {
                "namespace": "namespace",
                "region": "ap-hyderabad-1",
                "bucket_name": "bucket",
            },
        }))
        .expect("oci object storage config should deserialize");

        assert!(
            matches!(&file_storage, FileStorageConfig::OciObjectStorage { .. }),
            "{file_storage:?}"
        );
        assert!(file_storage.validate().is_ok());
    }

    #[test]
    fn rejects_values_that_do_not_fit_the_endpoint() {
        for (config, message) in [
            (
                config("", "ap-hyderabad-1", "bucket"),
                "oci object storage namespace",
            ),
            (
                config("name.space", "ap-hyderabad-1", "bucket"),
                "oci object storage namespace",
            ),
            (
                config("namespace", "", "bucket"),
                "oci object storage region",
            ),
            (
                config("namespace", "ap-hyderabad-1/x", "bucket"),
                "oci object storage region",
            ),
            (
                config("namespace", "ap-hyderabad-1", ""),
                "oci object storage bucket name",
            ),
        ] {
            let error = config.validate().err().map(|error| error.to_string());
            assert!(
                error
                    .as_deref()
                    .is_some_and(|error| error.starts_with(&format!("file_storage: {message}"))),
                "{config:?}: {error:?}"
            );
        }
    }

    #[tokio::test]
    async fn addresses_the_bucket_in_the_path_of_the_compatibility_endpoint() {
        let config = config("namespace", "ap-hyderabad-1", "bucket");

        let url = presigned_url(&config.endpoint(), &config.bucket_name, None).await;

        assert!(
            url.starts_with(
                "https://namespace.compat.objectstorage.ap-hyderabad-1.oraclecloud.com/bucket/files/file_key?"
            ),
            "{url}"
        );
    }

    #[tokio::test]
    async fn ignores_an_endpoint_from_the_environment() {
        let config = config("namespace", "ap-hyderabad-1", "bucket");

        let url = presigned_url(
            &config.endpoint(),
            &config.bucket_name,
            Some("https://endpoint.from.env"),
        )
        .await;

        assert!(
            url.starts_with(
                "https://namespace.compat.objectstorage.ap-hyderabad-1.oraclecloud.com/bucket/"
            ),
            "{url}"
        );
    }
}
