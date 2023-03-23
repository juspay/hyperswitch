use aws_config::meta::region::RegionProviderChain;
use aws_sdk_kms::{types::Blob, Client, Region};
use base64::Engine;
use error_stack::{IntoReport, ResultExt};

use crate::{
    configs::settings,
    consts,
    core::errors::{self, CustomResult},
    logger,
    routes::metrics,
};

static KMS_CLIENT: tokio::sync::OnceCell<KmsClient> = tokio::sync::OnceCell::const_new();

#[inline]
pub async fn get_kms_client(config: &settings::Kms) -> &KmsClient {
    KMS_CLIENT.get_or_init(|| KmsClient::new(config)).await
}

pub struct KmsClient {
    inner_client: Client,
    key_id: String,
}

impl KmsClient {
    /// Constructs a new KMS client.
    pub async fn new(config: &settings::Kms) -> Self {
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
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, errors::KmsError> {
        let data = consts::BASE64_ENGINE
            .decode(data)
            .into_report()
            .change_context(errors::KmsError::Base64DecodingFailed)?;
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
            .change_context(errors::KmsError::DecryptionFailed)?;

        decrypt_output
            .plaintext
            .ok_or(errors::KmsError::MissingPlaintextDecryptionOutput)
            .into_report()
            .and_then(|blob| {
                String::from_utf8(blob.into_inner())
                    .into_report()
                    .change_context(errors::KmsError::Utf8DecodingFailed)
            })
    }
}
