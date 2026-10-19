//! Google Cloud Storage, accessed through its XML API's Amazon S3 interoperability.

use super::{
    s3::{S3Endpoint, S3Storage},
    InvalidStorageConfig,
};

/// The XML API endpoint of Google Cloud Storage, the same for every bucket location.
const GCS_XML_API_ENDPOINT: &str = "https://storage.googleapis.com";

/// The signing region Google Cloud Storage accepts for buckets in any location.
const GCS_DEFAULT_SIGNING_REGION: &str = "auto";

/// Configuration for Google Cloud Storage.
///
/// Requests go to the XML API endpoint with the bucket in the path, since Google's TLS
/// certificate does not cover bucket names that contain dots. Authenticate with a Cloud Storage
/// HMAC key of a service account, through `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`.
#[derive(Debug, serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct GcsObjectStorageConfig {
    /// The bucket to send file uploads
    bucket_name: String,
    /// The region requests are signed for. Cloud Storage accepts `auto` for buckets in any
    /// location, which is used when this is unset.
    region: Option<String>,
}

impl GcsObjectStorageConfig {
    /// Validates the Google Cloud Storage configuration, returning a description of the first
    /// problem.
    pub fn validate(&self) -> Result<(), InvalidStorageConfig> {
        if self.bucket_name.trim().is_empty() {
            return Err(InvalidStorageConfig(
                "gcs object storage bucket name must not be empty",
            ));
        }

        if self
            .region
            .as_deref()
            .is_some_and(|region| region.trim().is_empty())
        {
            return Err(InvalidStorageConfig(
                "gcs object storage region must not be empty when set",
            ));
        }

        Ok(())
    }

    /// The region requests are signed for.
    fn signing_region(&self) -> &str {
        self.region.as_deref().unwrap_or(GCS_DEFAULT_SIGNING_REGION)
    }

    /// The XML API endpoint, addressed path-style.
    fn endpoint() -> S3Endpoint {
        S3Endpoint {
            url: Some(GCS_XML_API_ENDPOINT.to_owned()),
            force_path_style: true,
        }
    }
}

impl S3Storage {
    /// Creates a file storage client for Google Cloud Storage.
    pub async fn new_gcs(config: &GcsObjectStorageConfig) -> Self {
        Self::with_endpoint(
            config.signing_region(),
            &config.bucket_name,
            &GcsObjectStorageConfig::endpoint(),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::s3::tests::presigned_url;

    fn config(bucket_name: &str, region: Option<&str>) -> GcsObjectStorageConfig {
        GcsObjectStorageConfig {
            bucket_name: bucket_name.to_owned(),
            region: region.map(str::to_owned),
        }
    }

    #[test]
    fn validate_requires_a_bucket_and_a_non_empty_region_when_set() {
        assert_eq!(config("bucket", None).validate(), Ok(()));
        assert_eq!(config("bucket", Some("asia-south1")).validate(), Ok(()));
        assert_eq!(
            config(" ", None).validate(),
            Err(InvalidStorageConfig(
                "gcs object storage bucket name must not be empty"
            ))
        );
        assert_eq!(
            config("bucket", Some("")).validate(),
            Err(InvalidStorageConfig(
                "gcs object storage region must not be empty when set"
            ))
        );
    }

    #[test]
    fn signs_for_auto_unless_a_region_is_set() {
        assert_eq!(config("bucket", None).signing_region(), "auto");
        assert_eq!(
            config("bucket", Some("asia-south1")).signing_region(),
            "asia-south1"
        );
    }

    #[tokio::test]
    async fn addresses_the_bucket_in_the_path_of_the_xml_api_endpoint() {
        let url = presigned_url(&GcsObjectStorageConfig::endpoint(), "my.bucket", None).await;

        assert!(
            url.starts_with("https://storage.googleapis.com/my.bucket/files/file_key?"),
            "{url}"
        );
    }

    #[tokio::test]
    async fn ignores_an_endpoint_from_the_environment() {
        let url = presigned_url(
            &GcsObjectStorageConfig::endpoint(),
            "bucket",
            Some("https://endpoint.from.env"),
        )
        .await;

        assert!(
            url.starts_with("https://storage.googleapis.com/bucket/"),
            "{url}"
        );
    }
}
