//! OCI Object Storage, accessed through its Amazon S3 Compatibility API.

use super::s3::{S3Endpoint, S3Storage};

/// Configuration for OCI Object Storage.
///
/// Requests go to the S3 Compatibility API endpoint of the namespace and region, with the bucket
/// in the path, since OCI's TLS certificate does not cover bucket subdomains. Authenticate with an
/// OCI Customer Secret Key, through `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`.
#[derive(Debug, serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct OciObjectStorageConfig {
    /// The Object Storage namespace of the tenancy
    namespace: String,
    /// The OCI region identifier of the bucket, such as `us-ashburn-1`
    region: String,
    /// The bucket to send file uploads
    bucket_name: String,
}

impl OciObjectStorageConfig {
    /// Validates the OCI Object Storage configuration, returning a description of the first
    /// problem.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !is_hostname_label(&self.namespace) {
            return Err("oci object storage namespace must be non-empty and contain only letters, digits and hyphens");
        }

        if !is_hostname_label(&self.region) {
            return Err("oci object storage region must be non-empty and contain only letters, digits and hyphens");
        }

        if self.bucket_name.trim().is_empty() {
            return Err("oci object storage bucket name must not be empty");
        }

        Ok(())
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

impl S3Storage {
    /// Creates a file storage client for OCI Object Storage.
    pub async fn new_oci(config: &OciObjectStorageConfig) -> Self {
        Self::with_endpoint(&config.region, &config.bucket_name, &config.endpoint()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::s3::tests::presigned_url;

    fn config(namespace: &str, region: &str, bucket_name: &str) -> OciObjectStorageConfig {
        OciObjectStorageConfig {
            namespace: namespace.to_owned(),
            region: region.to_owned(),
            bucket_name: bucket_name.to_owned(),
        }
    }

    #[test]
    fn validate_rejects_values_that_do_not_fit_the_endpoint() {
        assert_eq!(
            config("namespace", "ap-hyderabad-1", "bucket").validate(),
            Ok(())
        );

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
                config("namespace", "ap-hyderabad-1", " "),
                "oci object storage bucket name",
            ),
        ] {
            let error = config.validate().err();
            assert!(
                error.is_some_and(|error| error.starts_with(message)),
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
