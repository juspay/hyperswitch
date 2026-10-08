//! S3 object storage, for AWS S3 and S3-compatible stores such as OCI Object Storage.

use aws_config::{meta::region::RegionProviderChain, Region};
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

/// Configuration for AWS S3 file storage.
#[derive(Debug, serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct S3StorageConfig {
    /// The AWS region to send file uploads
    region: String,
    /// The AWS s3 bucket to send file uploads
    bucket_name: String,
}

impl S3StorageConfig {
    /// Validates the S3 file storage configuration, returning a description of the first problem.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.region.trim().is_empty() {
            return Err("aws s3 region must not be empty");
        }

        if self.bucket_name.trim().is_empty() {
            return Err("aws s3 bucket name must not be empty");
        }

        Ok(())
    }
}

/// Where the S3 client sends requests, for S3-compatible stores other than AWS S3 itself.
#[derive(Debug, Clone, Default)]
pub(crate) struct S3Endpoint {
    /// Endpoint URL of the store. When unset, the endpoint is resolved from the region or
    /// `AWS_ENDPOINT_URL_S3`, as for AWS S3.
    pub(crate) url: Option<String>,
    /// Addresses buckets as `<endpoint>/<bucket>` rather than `<bucket>.<endpoint>`, for stores
    /// whose TLS certificate does not cover bucket subdomains.
    pub(crate) force_path_style: bool,
}

/// Builds the S3 client, applying the endpoint overrides on top of the SDK config. With the
/// default [`S3Endpoint`] this is the same client as `Client::new(sdk_config)`.
fn build_client(endpoint: &S3Endpoint, sdk_config: &aws_config::SdkConfig) -> Client {
    let mut s3_config = aws_sdk_s3::config::Builder::from(sdk_config);
    if let Some(url) = &endpoint.url {
        s3_config = s3_config.endpoint_url(url);
    }
    if endpoint.force_path_style {
        s3_config = s3_config.force_path_style(true);
    }
    Client::from_conf(s3_config.build())
}

/// S3 file storage client.
#[derive(Debug, Clone)]
pub struct S3Storage {
    /// AWS S3 client
    inner_client: Client,
    /// The name of the AWS S3 bucket.
    bucket_name: String,
}

impl S3Storage {
    /// Creates a new AWS S3 file storage client.
    pub async fn new(config: &S3StorageConfig) -> Self {
        Self::with_endpoint(&config.region, &config.bucket_name, &S3Endpoint::default()).await
    }

    /// Creates a client for an S3-compatible store reached through `endpoint`.
    pub(crate) async fn with_endpoint(
        region: &str,
        bucket_name: &str,
        endpoint: &S3Endpoint,
    ) -> Self {
        let region_provider = RegionProviderChain::first_try(Region::new(region.to_owned()));
        let sdk_config = aws_config::from_env().region(region_provider).load().await;
        Self {
            inner_client: build_client(endpoint, &sdk_config),
            bucket_name: bucket_name.to_owned(),
        }
    }

    /// Uploads a file to AWS S3.
    pub async fn upload_file(&self, file_key: &str, file: Vec<u8>) -> Result<(), S3StorageError> {
        self.inner_client
            .put_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .body(file.into())
            .send()
            .await
            .map_err(|error| S3StorageError::UploadFailure(Box::new(error)))?;
        Ok(())
    }

    /// Deletes a file from AWS S3.
    pub async fn delete_file(&self, file_key: &str) -> Result<(), S3StorageError> {
        self.inner_client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .send()
            .await
            .map_err(|error| S3StorageError::DeleteFailure(Box::new(error)))?;
        Ok(())
    }

    /// Retrieves a file from AWS S3.
    pub async fn retrieve_file(&self, file_key: &str) -> Result<Vec<u8>, S3StorageError> {
        Ok(self
            .inner_client
            .get_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .send()
            .await
            .map_err(|error| S3StorageError::RetrieveFailure(Box::new(error)))?
            .body
            .collect()
            .await
            .map_err(S3StorageError::UnknownError)?
            .to_vec())
    }

    /// Begins a multipart upload on AWS S3.
    pub async fn create_multipart_upload(&self, file_key: &str) -> Result<String, S3StorageError> {
        self.inner_client
            .create_multipart_upload()
            .bucket(&self.bucket_name)
            .key(file_key)
            .content_type("text/csv")
            .send()
            .await
            .map_err(|error| S3StorageError::MultipartCreateFailure(Box::new(error)))?
            .upload_id
            .ok_or(S3StorageError::MissingUploadId)
    }

    /// Uploads a single part of an in-progress multipart upload to AWS S3.
    pub async fn upload_part(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        body: Vec<u8>,
    ) -> Result<String, S3StorageError> {
        self.inner_client
            .upload_part()
            .bucket(&self.bucket_name)
            .key(file_key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(body.into())
            .send()
            .await
            .map_err(|error| S3StorageError::MultipartUploadPartFailure(Box::new(error)))?
            .e_tag
            .ok_or(S3StorageError::MissingPartETag)
    }

    /// Assembles the uploaded parts into a single S3 object, in part number order.
    pub async fn complete_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
        parts: Vec<(i32, String)>,
    ) -> Result<Option<String>, S3StorageError> {
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
            .map_err(|error| S3StorageError::MultipartCompleteFailure(Box::new(error)))?
            .expiration)
    }

    /// Discards an in-progress multipart upload and the parts already uploaded for it.
    pub async fn abort_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
    ) -> Result<(), S3StorageError> {
        self.inner_client
            .abort_multipart_upload()
            .bucket(&self.bucket_name)
            .key(file_key)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(|error| S3StorageError::MultipartAbortFailure(Box::new(error)))?;
        Ok(())
    }

    /// Signs a time-limited GET for the object with the client's current credentials.
    pub async fn get_presigned_download_url(
        &self,
        file_key: &str,
        expires_in: std::time::Duration,
        download_file_name: Option<&str>,
    ) -> Result<url::Url, S3StorageError> {
        let presigning_config = PresigningConfig::expires_in(expires_in)
            .map_err(S3StorageError::PresigningConfigFailure)?;

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
            .map_err(|error| S3StorageError::PresignFailure(Box::new(error)))?;

        url::Url::parse(presigned.uri()).map_err(S3StorageError::PresignedUrlParseFailure)
    }
}

/// Errors that can occur during S3 file storage operations.
#[derive(Debug, thiserror::Error)]
pub enum S3StorageError {
    /// Error indicating that file upload to S3 failed.
    #[error("File upload to S3 failed: {0:?}")]
    UploadFailure(Box<aws_sdk_s3::error::SdkError<PutObjectError>>),

    /// Error indicating that file retrieval from S3 failed.
    #[error("File retrieve from S3 failed: {0:?}")]
    RetrieveFailure(Box<aws_sdk_s3::error::SdkError<GetObjectError>>),

    /// Error indicating that file deletion from S3 failed.
    #[error("File delete from S3 failed: {0:?}")]
    DeleteFailure(Box<aws_sdk_s3::error::SdkError<DeleteObjectError>>),

    /// Unknown error occurred.
    #[error("Unknown error occurred: {0:?}")]
    UnknownError(aws_sdk_s3::primitives::ByteStreamError),

    /// Error indicating that starting a multipart upload on S3 failed.
    #[error("Multipart upload create on S3 failed: {0:?}")]
    MultipartCreateFailure(Box<aws_sdk_s3::error::SdkError<CreateMultipartUploadError>>),

    /// Error indicating that uploading a part to S3 failed.
    #[error("Multipart part upload to S3 failed: {0:?}")]
    MultipartUploadPartFailure(Box<aws_sdk_s3::error::SdkError<UploadPartError>>),

    /// Error indicating that assembling a multipart upload on S3 failed.
    #[error("Multipart upload complete on S3 failed: {0:?}")]
    MultipartCompleteFailure(Box<aws_sdk_s3::error::SdkError<CompleteMultipartUploadError>>),

    /// Error indicating that discarding a multipart upload on S3 failed.
    #[error("Multipart upload abort on S3 failed: {0:?}")]
    MultipartAbortFailure(Box<aws_sdk_s3::error::SdkError<AbortMultipartUploadError>>),

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
    PresignFailure(Box<aws_sdk_s3::error::SdkError<GetObjectError>>),

    /// Error indicating the presigned URI could not be parsed as a URL.
    #[error("Presigned URI is not a valid URL: {0}")]
    PresignedUrlParseFailure(url::ParseError),
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;

    use aws_sdk_s3::config::{BehaviorVersion, Credentials, SharedCredentialsProvider};

    use super::*;

    fn sdk_config(endpoint_url: Option<&str>) -> aws_config::SdkConfig {
        let builder = aws_config::SdkConfig::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("ap-hyderabad-1"))
            .credentials_provider(SharedCredentialsProvider::new(Credentials::new(
                "access_key",
                "secret_key",
                None,
                None,
                "test",
            )));
        match endpoint_url {
            Some(endpoint_url) => builder.endpoint_url(endpoint_url).build(),
            None => builder.build(),
        }
    }

    /// Presigns a download of `files/file_key`, which shows where the client sends requests
    /// without any network call.
    pub(crate) async fn presigned_url(
        endpoint: &S3Endpoint,
        bucket_name: &str,
        sdk_endpoint_url: Option<&str>,
    ) -> String {
        #[allow(clippy::expect_used)]
        let presigning_config =
            PresigningConfig::expires_in(Duration::from_secs(60)).expect("valid presigning config");

        #[allow(clippy::expect_used)]
        build_client(endpoint, &sdk_config(sdk_endpoint_url))
            .get_object()
            .bucket(bucket_name)
            .key("files/file_key")
            .presigned(presigning_config)
            .await
            .expect("presigning is local and should succeed")
            .uri()
            .to_owned()
    }

    #[test]
    fn validate_requires_region_and_bucket() {
        let config = |region: &str, bucket_name: &str| S3StorageConfig {
            region: region.to_owned(),
            bucket_name: bucket_name.to_owned(),
        };

        assert_eq!(config("us-east-1", "bucket").validate(), Ok(()));
        assert_eq!(
            config(" ", "bucket").validate(),
            Err("aws s3 region must not be empty")
        );
        assert_eq!(
            config("us-east-1", "").validate(),
            Err("aws s3 bucket name must not be empty")
        );
    }

    #[tokio::test]
    async fn aws_config_uses_virtual_hosted_aws_endpoint() {
        let url = presigned_url(&S3Endpoint::default(), "bucket", None).await;

        assert!(
            url.starts_with("https://bucket.s3.ap-hyderabad-1.amazonaws.com/files/file_key?"),
            "{url}"
        );
    }

    #[tokio::test]
    async fn endpoint_from_environment_is_kept_when_not_overridden() {
        let endpoint = S3Endpoint {
            url: None,
            force_path_style: true,
        };

        let url = presigned_url(&endpoint, "bucket", Some("https://endpoint.from.env")).await;

        assert!(
            url.starts_with("https://endpoint.from.env/bucket/files/file_key?"),
            "{url}"
        );
    }
}
