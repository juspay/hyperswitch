//! Google Cloud Storage, accessed through its XML API's Amazon S3 interoperability.

use aws_sdk_s3::config::Credentials;

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
/// certificate does not cover bucket names that contain dots. They are signed with a Cloud
/// Storage HMAC key of a service account, taken from this configuration rather than the AWS
/// credential chain, so the process can still use AWS services with its own credentials.
#[derive(serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct GcsObjectStorageConfig {
    /// The bucket to send file uploads
    bucket_name: String,
    /// The access ID of the service account's HMAC key, such as `GOOG1E...`
    hmac_access_id: String,
    /// The secret of the service account's HMAC key
    hmac_secret: String,
    /// The region requests are signed for. Cloud Storage accepts `auto` for buckets in any
    /// location, which is used when this is unset.
    region: Option<String>,
}

impl std::fmt::Debug for GcsObjectStorageConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GcsObjectStorageConfig")
            .field("bucket_name", &self.bucket_name)
            .field("hmac_access_id", &self.hmac_access_id)
            .field("hmac_secret", &"***")
            .field("region", &self.region)
            .finish()
    }
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

        if self.hmac_access_id.trim().is_empty() {
            return Err(InvalidStorageConfig(
                "gcs object storage hmac access id must not be empty",
            ));
        }

        if self.hmac_secret.trim().is_empty() {
            return Err(InvalidStorageConfig(
                "gcs object storage hmac secret must not be empty",
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

    /// The HMAC key requests are signed with.
    fn credentials(&self) -> Credentials {
        Credentials::new(
            &self.hmac_access_id,
            &self.hmac_secret,
            None,
            None,
            "gcs_object_storage",
        )
    }
}

impl S3Storage {
    /// Creates a file storage client for Google Cloud Storage.
    pub async fn new_gcs(config: &GcsObjectStorageConfig) -> Self {
        Self::with_endpoint(
            config.signing_region(),
            &config.bucket_name,
            &GcsObjectStorageConfig::endpoint(),
            Some(config.credentials()),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::storage::s3::tests::presigned_url;

    fn config(bucket_name: &str, region: Option<&str>) -> GcsObjectStorageConfig {
        GcsObjectStorageConfig {
            bucket_name: bucket_name.to_owned(),
            hmac_access_id: "GOOG1EXAMPLEACCESSID".to_owned(),
            hmac_secret: "example-hmac-secret".to_owned(),
            region: region.map(str::to_owned),
        }
    }

    #[test]
    fn validate_requires_a_bucket_an_hmac_key_and_a_non_empty_region_when_set() {
        assert_eq!(config("bucket", None).validate(), Ok(()));
        assert_eq!(config("bucket", Some("asia-south1")).validate(), Ok(()));
        assert_eq!(
            config(" ", None).validate(),
            Err(InvalidStorageConfig(
                "gcs object storage bucket name must not be empty"
            ))
        );
        assert_eq!(
            GcsObjectStorageConfig {
                hmac_access_id: String::new(),
                ..config("bucket", None)
            }
            .validate(),
            Err(InvalidStorageConfig(
                "gcs object storage hmac access id must not be empty"
            ))
        );
        assert_eq!(
            GcsObjectStorageConfig {
                hmac_secret: " ".to_owned(),
                ..config("bucket", None)
            }
            .validate(),
            Err(InvalidStorageConfig(
                "gcs object storage hmac secret must not be empty"
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
    fn debug_redacts_the_hmac_secret() {
        let debug = format!("{:?}", config("bucket", None));

        assert!(debug.contains("GOOG1EXAMPLEACCESSID"), "{debug}");
        assert!(!debug.contains("example-hmac-secret"), "{debug}");
    }

    #[tokio::test]
    async fn signs_with_the_configured_hmac_key() {
        #[allow(clippy::expect_used)]
        let url = S3Storage::new_gcs(&config("bucket", None))
            .await
            .get_presigned_download_url("files/file_key", Duration::from_secs(60), None)
            .await
            .expect("presigning is local and should succeed");

        assert!(
            url.as_str()
                .starts_with("https://storage.googleapis.com/bucket/files/file_key?"),
            "{url}"
        );
        assert!(
            url.as_str()
                .contains("X-Amz-Credential=GOOG1EXAMPLEACCESSID%2F"),
            "{url}"
        );
        assert!(
            url.as_str().contains("%2Fauto%2Fs3%2Faws4_request"),
            "{url}"
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
