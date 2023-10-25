//! Interactions with the AWS KMS SDK

use std::time::Instant;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_kms::{config::Region, primitives::Blob, Client};
use base64::Engine;
use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use masking::{PeekInterface, Secret};
use router_env::logger;
/// decrypting data using the AWS KMS SDK.
pub mod decrypt;

use crate::{consts, metrics};

static KMS_CLIENT: tokio::sync::OnceCell<KmsClient> = tokio::sync::OnceCell::const_new();

/// Returns a shared KMS client, or initializes a new one if not previously initialized.
#[inline]
pub async fn get_kms_client(config: &KmsConfig) -> &'static KmsClient {
    KMS_CLIENT.get_or_init(|| KmsClient::new(config)).await
}

/// Configuration parameters required for constructing a [`KmsClient`].
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct KmsConfig {
    /// The AWS key identifier of the KMS key used to encrypt or decrypt data.
    pub key_id: String,

    /// The AWS region to send KMS requests to.
    pub region: String,
}

/// Client for KMS operations.
#[derive(Debug)]
pub struct KmsClient {
    inner_client: Client,
    key_id: String,
}

impl KmsClient {
    /// Constructs a new KMS client.
    pub async fn new(config: &KmsConfig) -> Self {
        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));
        let sdk_config = aws_config::from_env().region(region_provider).load().await;

        Self {
            inner_client: Client::new(&sdk_config),
            key_id: config.key_id.clone(),
        }
    }

    /// Decrypts the provided base64-encoded encrypted data using the AWS KMS SDK. We assume that
    /// the SDK has the values required to interact with the AWS KMS APIs (`AWS_ACCESS_KEY_ID` and
    /// `AWS_SECRET_ACCESS_KEY`) either set in environment variables, or that the SDK is running in
    /// a machine that is able to assume an IAM role.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, KmsError> {
        let start = Instant::now();
        let data = consts::BASE64_ENGINE
            .decode(data)
            .into_report()
            .change_context(KmsError::Base64DecodingFailed)?;
        let ciphertext_blob = Blob::new(data);

        let decrypt_output = self
            .inner_client
            .decrypt()
            .key_id(&self.key_id)
            .ciphertext_blob(ciphertext_blob)
            .send()
            .await
            .map_err(|error| {
                // Logging using `Debug` representation of the error as the `Display`
                // representation does not hold sufficient information.
                logger::error!(kms_sdk_error=?error, "Failed to KMS decrypt data");
                metrics::AWS_KMS_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                error
            })
            .into_report()
            .change_context(KmsError::DecryptionFailed)?;

        let output = decrypt_output
            .plaintext
            .ok_or(KmsError::MissingPlaintextDecryptionOutput)
            .into_report()
            .and_then(|blob| {
                String::from_utf8(blob.into_inner())
                    .into_report()
                    .change_context(KmsError::Utf8DecodingFailed)
            })?;

        let time_taken = start.elapsed();
        metrics::AWS_KMS_DECRYPT_TIME.record(&metrics::CONTEXT, time_taken.as_secs_f64(), &[]);

        Ok(output)
    }
}

/// Errors that could occur during KMS operations.
#[derive(Debug, thiserror::Error)]
pub enum KmsError {
    /// An error occurred when base64 decoding input data.
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,

    /// An error occurred when KMS decrypting input data.
    #[error("Failed to KMS decrypt input data")]
    DecryptionFailed,

    /// The KMS decrypted output does not include a plaintext output.
    #[error("Missing plaintext KMS decryption output")]
    MissingPlaintextDecryptionOutput,

    /// An error occurred UTF-8 decoding KMS decrypted output.
    #[error("Failed to UTF-8 decode decryption output")]
    Utf8DecodingFailed,

    /// The KMS client has not been initialized.
    #[error("The KMS client has not been initialized")]
    KmsClientNotInitialized,
}

impl KmsConfig {
    /// Verifies that the [`KmsClient`] configuration is usable.
    pub fn validate(&self) -> Result<(), &'static str> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};

        when(self.key_id.is_default_or_empty(), || {
            Err("KMS AWS key ID must not be empty")
        })?;

        when(self.region.is_default_or_empty(), || {
            Err("KMS AWS region must not be empty")
        })
    }
}

/// A wrapper around a KMS value that can be decrypted.
#[derive(Clone, Debug, Default, serde::Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct KmsValue(Secret<String>);

impl common_utils::ext_traits::ConfigExt for KmsValue {
    fn is_empty_after_trim(&self) -> bool {
        self.0.peek().is_empty_after_trim()
    }
}
