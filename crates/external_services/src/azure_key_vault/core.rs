//! Interactions with the AZURE KEY VAULT SDK

use std::sync::Arc;

use azure_identity::DefaultAzureCredential;
use azure_security_keyvault_keys::{
    KeyClient,
    models::{KeyOperationsParameters, JsonWebKeyEncryptionAlgorithm},
};
use crate::{consts, metrics};
use base64::Engine;

use std::time::Instant;
use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use router_env::logger;


/// Configuration parameters required for constructing a [`AzureKeyVaultClient`].
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct AzureKeyVaultConfig {
    /// key name of Azure Key vault used to encrypt or decrypt data
    pub key_name: String,
    /// The Azure vault url of the Key vault.
    pub vault_url: String,
    /// version of the key name
    pub version: String,
}

impl AzureKeyVaultConfig {
    /// Verifies that the [`AzureKeyVaultClient`] configuration is usable.
    pub fn validate(&self) -> Result<(), &'static str> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};

        when(self.key_name.is_default_or_empty(), || {
            Err("Azure Key Vault key name must not be empty")
        })?;

        when(self.vault_url.is_default_or_empty(), || {
            Err("Azure Key Vault url must not be empty")
        })
    }
}

/// Client for AZURE KEY VAULT operations.
#[derive(Clone)]
pub struct AzureKeyVaultClient {
    inner_client: Arc<KeyClient>,
    key_name: String,
    version: String,
}

impl AzureKeyVaultClient {
    /// Constructs a new Azure Key Vault client.
    pub async fn new(config: &AzureKeyVaultConfig) -> Result<Self, AzureKeyVaultError> {
        let credential = DefaultAzureCredential::new()
            .map_err(|_| AzureKeyVaultError::AzureKeyVaultClientInitializationFailed)?;

        Ok(Self {
            inner_client: Arc::new(
                    KeyClient::new(
                    &config.vault_url,
                    credential.clone(),
                    None
                )
                .map_err(
                    |_| AzureKeyVaultError::AzureKeyVaultClientInitializationFailed)?
            ),
            key_name: config.key_name.clone(),
            version: config.version.clone(),
        })
    }
    /// Decrypts the provided base64-encoded encrypted data using the AZURE KEY VAULT SDK. We assume that
    /// the SDK has the values required to interact with the AZURE KEY VAULT APIs (`AZURE_TENANT_ID`,
    /// `AZURE_CLIENT_ID` and `AZURE_CLIENT_SECRET`) either set in environment variables, or that the
    /// SDK is running in a machine that is able to assume an Azure AD role.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, AzureKeyVaultError> {
        let start = Instant::now();

        let data = consts::BASE64_ENGINE
            .decode(data)
            .change_context(AzureKeyVaultError::Base64DecodingFailed)?;

        let decrypt_params = KeyOperationsParameters {
            algorithm: Some(JsonWebKeyEncryptionAlgorithm::RsaOaep),
            value: Some(data),
            ..Default::default()
        };
        let decrypted_output = self.inner_client
            .decrypt(&self.key_name, &self.version , decrypt_params.clone().try_into().unwrap(), None)
            .await
            .inspect_err(|error| {
                logger::error!(azure_key_vault_error=?error, "Failed to Azure Key Vault decrypt data");
                metrics::AZURE_KEY_VAULT_DECRYPTION_FAILURES.add(1, &[]);
            })
            .change_context(AzureKeyVaultError::DecryptionFailed)?
            .into_body()
            .await
            .inspect_err(|error| {
                logger::error!(azure_key_vault_error=?error, "Failed to Azure Key Vault decrypt data");
                metrics::AZURE_KEY_VAULT_DECRYPTION_FAILURES.add(1, &[]);
            })
            .change_context(AzureKeyVaultError::DecryptionFailed)?;

        let output = decrypted_output
            .result
            .ok_or(report!(AzureKeyVaultError::MissingPlaintextDecryptionOutput))
            .and_then(|bytes|
                String::from_utf8(bytes)
                    .change_context(AzureKeyVaultError::Utf8DecodingFailed)
            )?;

        let time_taken = start.elapsed();
        metrics::AZURE_KEY_VAULT_DECRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }

    /// Encrypts the provided String using the AZURE KEY VAULT SDK and returns base64-encoded encrypted data.
    ///  We assume that the SDK has the values required to interact with the AZURE KEY VAULT APIs (`AZURE_TENANT_ID`,
    /// `AZURE_CLIENT_ID` and `AZURE_CLIENT_SECRET`) either set in environment variables, or that the
    /// SDK is running in a machine that is able to assume an Azure AD role.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, AzureKeyVaultError> {
        let start = Instant::now();

        let encrypt_params = KeyOperationsParameters {
            algorithm: Some(JsonWebKeyEncryptionAlgorithm::RsaOaep),
            value: Some(data.as_ref().to_vec()),
            ..Default::default()
        };

        let encrypted_output = self
            .inner_client
            .encrypt(&self.key_name, &self.version, encrypt_params.clone().try_into().unwrap(), None)
            .await
            .inspect_err(|error| {
                logger::error!(azure_key_vault_error=?error, "Failed to Azure Key Vault decrypt data");
                metrics::AZURE_KEY_VAULT_ENCRYPTION_FAILURES.add(1, &[]);
            })
            .change_context(AzureKeyVaultError::EncryptionFailed)?
            .into_body()
            .await
            .inspect_err(|error| {
                logger::error!(azure_key_vault_error=?error, "Failed to Azure Key Vault decrypt data");
                metrics::AZURE_KEY_VAULT_ENCRYPTION_FAILURES.add(1, &[]);
            })
            .change_context(AzureKeyVaultError::EncryptionFailed)?;

        let output = encrypted_output
            .result
            .ok_or(AzureKeyVaultError::MissingCiphertextEncryptionOutput)
            .map(|bytes| consts::BASE64_ENGINE.encode(bytes))?;

        let time_taken = start.elapsed();
        metrics::AZURE_KEY_VAULT_ENCRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }


}


/// Errors that could occur during AZURE KEY VAULT operations.
#[derive(Debug, thiserror::Error)]
pub enum AzureKeyVaultError {
    /// An error occurred when base64 encoding input data.
    #[error("Failed to base64 encode input data")]
    Base64EncodingFailed,

    /// An error occurred when base64 decoding input data.
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,

    /// An error occurred when AZURE KEY VAULT decrypting input data.
    #[error("Failed to Azure Key Vault decrypt input data")]
    DecryptionFailed,

    /// An error occurred when AZURE KEY VAULT encrypting input data.
    #[error("Failed to Azure Key Vault encrypt input data")]
    EncryptionFailed,

    /// The AZURE KEY VAULT decrypted output does not include a plaintext output.
    #[error("Missing plaintext AZURE KEY VAULT decryption output")]
    MissingPlaintextDecryptionOutput,

    /// The AZURE KEY VAULT encrypted output does not include a ciphertext output.
    #[error("Missing ciphertext AZURE KEY VAULT encryption output")]
    MissingCiphertextEncryptionOutput,

    /// An error occurred UTF-8 decoding AZURE KEY VAULT decrypted output.
    #[error("Failed to UTF-8 decode decryption output")]
    Utf8DecodingFailed,

    /// The AZURE KEY VAULT client has not been initialized.
    #[error("The AZURE KEY VAULT client has not been initialized")]
    AzureKeyVaultClientInitializationFailed,
}


#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::print_stdout)]
    #[tokio::test]
    async fn check_azure_key_vault_encryption() {
        std::env::set_var("AZURE_CLIENT_ID", "YOUR-CLIENT-ID");
        std::env::set_var("AZURE_TENANT_ID", "YOUR-TENANT-ID");
        std::env::set_var("AZURE_CLIENT_SECRET", "YOUR-CLIENT-SECRET");
        use super::*;
        let config = AzureKeyVaultConfig {
            key_name: "YOUR AZURE KEY VAULT KEY NAME".to_string(),
            vault_url: "YOUR AZURE KEY VAULT URL".to_string(),
            version: "".to_string(),
        };

        let data = "hello".to_string();
        let binding = data.as_bytes();
        let encrypted_fingerprint = AzureKeyVaultClient::new(&config)
            .await
            .expect("azure key vault client initialization failed")
            .encrypt(binding)
            .await
            .expect("azure key vault encryption failed");

        println!("{}", encrypted_fingerprint);
    }

    #[tokio::test]
    async fn check_azure_key_vault_decrypt() {
        std::env::set_var("AZURE_CLIENT_ID", "YOUR-CLIENT-ID");
        std::env::set_var("AZURE_TENANT_ID", "YOUR-TENANT-ID");
        std::env::set_var("AZURE_CLIENT_SECRET", "YOUR-CLIENT-SECRET");
        use super::*;
        let config = AzureKeyVaultConfig {
            key_name: "YOUR AZURE KEY VAULT KEY NAME".to_string(),
            vault_url: "YOUR AZURE KEY VAULT URL".to_string(),
            version: "".to_string(),
        };

        // Should decrypt to hello
        let data = "AZURE KEY VAULT ENCRYPTED CIPHER".to_string();
        let binding = data.as_bytes();
        let decrypted_fingerprint = AzureKeyVaultClient::new(&config)
            .await
            .expect("azure key vault client initialization failed")
            .encrypt(binding)
            .await
            .expect("azure key vault decryption failed");

        println!("{}", decrypted_fingerprint);
    }
}
