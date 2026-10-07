//! The OCI Vault KMS crypto-endpoint client.

use std::sync::Arc;

use base64::Engine;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::{
    config::OciKmsConfig,
    credentials::CredentialCache,
    environment::{Environment, SystemEnvironment},
    error::OciKmsError,
    signing,
    transport::{self, AttemptError},
};

const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

const ENCRYPT_PATH: &str = "/20180608/encrypt";
const DECRYPT_PATH: &str = "/20180608/decrypt";
const GENERATE_DATA_KEY_PATH: &str = "/20180608/generateDataEncryptionKey";

const ENCRYPTION_ALGORITHM: &str = "AES_256_GCM";
const DATA_KEY_LENGTH: usize = 32;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EncryptDataDetails<'a> {
    key_id: &'a str,
    plaintext: String,
    encryption_algorithm: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DecryptDataDetails<'a> {
    key_id: &'a str,
    ciphertext: &'a str,
    encryption_algorithm: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateKeyDetails<'a> {
    key_id: &'a str,
    include_plaintext_key: bool,
    key_shape: KeyShape,
}

#[derive(Serialize)]
struct KeyShape {
    algorithm: &'static str,
    length: usize,
}

#[derive(Deserialize)]
struct EncryptedData {
    ciphertext: String,
}

#[derive(Deserialize)]
struct DecryptedData {
    plaintext: String,
}

#[derive(Deserialize)]
struct GeneratedKey {
    ciphertext: String,
    plaintext: Option<String>,
}

/// A fresh AES-256 data key from [`OciKmsClient::generate_data_key`], for envelope encryption.
///
/// The plaintext key is zeroed on drop and redacted from `Debug`.
pub struct DataKey {
    plaintext: [u8; DATA_KEY_LENGTH],
    ciphertext: String,
}

impl DataKey {
    /// The plaintext key, to encrypt data with locally. Don't store it.
    pub fn plaintext(&self) -> &[u8; DATA_KEY_LENGTH] {
        &self.plaintext
    }

    /// The same key, encrypted under the vault key. Store this, and recover the plaintext
    /// later with [`OciKmsClient::decrypt`].
    pub fn ciphertext(&self) -> &str {
        &self.ciphertext
    }
}

impl Drop for DataKey {
    fn drop(&mut self) {
        self.plaintext.zeroize();
    }
}

impl std::fmt::Debug for DataKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataKey")
            .field("plaintext", &"<redacted>")
            .field("ciphertext", &self.ciphertext)
            .finish()
    }
}

/// Client for OCI Vault KMS crypto operations.
///
/// Cheap to clone: clones share the HTTP connection pool and the cached credentials.
/// Credentials are resolved on first use, not at construction.
#[derive(Clone)]
pub struct OciKmsClient {
    http_client: reqwest::Client,
    vault_crypto_endpoint: String,
    host: String,
    key_id: String,
    credentials: Arc<CredentialCache>,
    environment: Arc<dyn Environment>,
}

impl std::fmt::Debug for OciKmsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OciKmsClient")
            .field("vault_crypto_endpoint", &self.vault_crypto_endpoint)
            .field("key_id", &self.key_id)
            .finish()
    }
}

impl OciKmsClient {
    /// Constructs a client that reads the system clock and RNG directly.
    pub fn new(config: &OciKmsConfig) -> Result<Self, OciKmsError> {
        Self::with_environment(config, Arc::new(SystemEnvironment))
    }

    /// Constructs a client whose clock and entropy reads go through `environment`.
    pub fn with_environment(
        config: &OciKmsConfig,
        environment: Arc<dyn Environment>,
    ) -> Result<Self, OciKmsError> {
        let host = url::Url::parse(&config.vault_crypto_endpoint)
            .map_err(|error| {
                OciKmsError::InvalidConfig(format!("invalid vault crypto endpoint URL: {error}"))
            })?
            .host_str()
            .ok_or_else(|| {
                OciKmsError::InvalidConfig("vault crypto endpoint URL has no host".to_owned())
            })?
            .to_owned();

        let http_client = transport::client_builder()
            .build()
            .map_err(|error| OciKmsError::ClientCreationFailed(error.to_string()))?;

        Ok(Self {
            http_client,
            vault_crypto_endpoint: config
                .vault_crypto_endpoint
                .trim_end_matches('/')
                .to_owned(),
            host,
            key_id: config.key_id.clone(),
            credentials: Arc::new(CredentialCache::default()),
            environment,
        })
    }

    /// Encrypts `plaintext` (at most 4 KiB) under the vault key, returning OCI's base64
    /// ciphertext.
    pub async fn encrypt(&self, plaintext: &[u8]) -> Result<String, OciKmsError> {
        let request = EncryptDataDetails {
            key_id: &self.key_id,
            plaintext: BASE64_ENGINE.encode(plaintext),
            encryption_algorithm: ENCRYPTION_ALGORITHM,
        };
        let response: EncryptedData = self.call(ENCRYPT_PATH, &request).await?;
        Ok(response.ciphertext)
    }

    /// Decrypts OCI base64 `ciphertext`: from [`Self::encrypt`], [`DataKey::ciphertext`], or
    /// `oci kms crypto encrypt`.
    pub async fn decrypt(&self, ciphertext: &str) -> Result<Vec<u8>, OciKmsError> {
        let request = DecryptDataDetails {
            key_id: &self.key_id,
            ciphertext,
            encryption_algorithm: ENCRYPTION_ALGORITHM,
        };
        let response: DecryptedData = self.call(DECRYPT_PATH, &request).await?;
        BASE64_ENGINE.decode(response.plaintext).map_err(|error| {
            OciKmsError::InvalidResponse(format!("plaintext is not valid base64: {error}"))
        })
    }

    /// Generates a fresh AES-256 data key, returned both in plaintext and encrypted under the
    /// vault key.
    pub async fn generate_data_key(&self) -> Result<DataKey, OciKmsError> {
        let request = GenerateKeyDetails {
            key_id: &self.key_id,
            include_plaintext_key: true,
            key_shape: KeyShape {
                algorithm: "AES",
                length: DATA_KEY_LENGTH,
            },
        };
        let response: GeneratedKey = self.call(GENERATE_DATA_KEY_PATH, &request).await?;

        let mut decoded = BASE64_ENGINE
            .decode(response.plaintext.unwrap_or_default())
            .map_err(|error| {
                OciKmsError::InvalidResponse(format!("data key is not valid base64: {error}"))
            })?;
        let plaintext = <[u8; DATA_KEY_LENGTH]>::try_from(decoded.as_slice()).map_err(|_| {
            OciKmsError::InvalidResponse(format!(
                "data key is {} bytes, expected {DATA_KEY_LENGTH}",
                decoded.len()
            ))
        });
        decoded.zeroize();

        Ok(DataKey {
            plaintext: plaintext?,
            ciphertext: response.ciphertext,
        })
    }

    async fn call<Request, Response>(
        &self,
        path: &'static str,
        request: &Request,
    ) -> Result<Response, OciKmsError>
    where
        Request: Serialize,
        Response: serde::de::DeserializeOwned,
    {
        let body = serde_json::to_vec(request).map_err(|error| {
            OciKmsError::RequestFailed(format!("failed to serialize the request body: {error}"))
        })?;

        let response_body = transport::with_retries(self.environment.as_ref(), path, || {
            self.send_once(path, &body)
        })
        .await?;

        serde_json::from_str(&response_body).map_err(|error| {
            OciKmsError::InvalidResponse(format!("failed to parse the response body: {error}"))
        })
    }

    /// One signed attempt. Signed afresh on every call, since the signature covers `date`.
    async fn send_once(&self, path: &str, body: &[u8]) -> Result<String, AttemptError> {
        // Credential resolution retries on its own; a failure surfacing here is final.
        let credentials = self
            .credentials
            .current(self.environment.as_ref())
            .await
            .map_err(AttemptError::Fatal)?;

        let signed = signing::sign_post_request(
            self.environment.as_ref(),
            &credentials.key_id,
            &credentials.private_key,
            &self.host,
            path,
            body,
        )
        .map_err(AttemptError::Fatal)?;

        let response = self
            .http_client
            .post(format!("{}{path}", self.vault_crypto_endpoint))
            .header("date", signed.date)
            .header("authorization", signed.authorization)
            .header("content-type", "application/json")
            .header("x-content-sha256", signed.x_content_sha256)
            .header(
                "opc-request-id",
                transport::request_id(self.environment.as_ref()),
            )
            .body(body.to_vec())
            .send()
            .await
            .map_err(|error| {
                AttemptError::Retryable(OciKmsError::RequestFailed(format!(
                    "failed to send the request: {error}"
                )))
            })?;

        let status = response.status();
        let response_body = response.text().await.map_err(|error| {
            AttemptError::Retryable(OciKmsError::RequestFailed(format!(
                "failed to read the response body: {error}"
            )))
        })?;

        if !status.is_success() {
            return Err(AttemptError::from_status(
                status,
                OciKmsError::UnexpectedStatus {
                    status: status.as_u16(),
                    body: response_body,
                },
            ));
        }

        Ok(response_body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> OciKmsConfig {
        OciKmsConfig {
            vault_crypto_endpoint: "https://abc-crypto.kms.ap-mumbai-1.oci.oraclecloud.com"
                .to_string(),
            key_id: "ocid1.key.oc1.ap-mumbai-1.test".to_string(),
        }
    }

    #[test]
    fn new_extracts_host_and_trims_trailing_slash() {
        let client = OciKmsClient::new(&OciKmsConfig {
            vault_crypto_endpoint: "https://abc-crypto.kms.ap-mumbai-1.oci.oraclecloud.com/"
                .to_string(),
            ..config()
        })
        .expect("client should build from a valid endpoint");

        assert_eq!(
            client.host,
            "abc-crypto.kms.ap-mumbai-1.oci.oraclecloud.com"
        );
        assert_eq!(
            client.vault_crypto_endpoint,
            "https://abc-crypto.kms.ap-mumbai-1.oci.oraclecloud.com"
        );
    }

    #[test]
    fn new_rejects_an_invalid_endpoint() {
        let result = OciKmsClient::new(&OciKmsConfig {
            vault_crypto_endpoint: "not a url".to_string(),
            ..config()
        });
        assert!(matches!(result, Err(OciKmsError::InvalidConfig(_))));
    }

    #[test]
    fn data_key_debug_redacts_the_plaintext() {
        let key = DataKey {
            plaintext: [7; DATA_KEY_LENGTH],
            ciphertext: "wrapped".to_owned(),
        };
        let debug = format!("{key:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains('7'));
    }

    #[test]
    fn requests_serialize_with_ocis_camel_case_field_names() {
        let body = serde_json::to_value(GenerateKeyDetails {
            key_id: "k",
            include_plaintext_key: true,
            key_shape: KeyShape {
                algorithm: "AES",
                length: 32,
            },
        })
        .expect("serializable");
        assert_eq!(
            body,
            serde_json::json!({
                "keyId": "k",
                "includePlaintextKey": true,
                "keyShape": { "algorithm": "AES", "length": 32 }
            })
        );

        let body = serde_json::to_value(DecryptDataDetails {
            key_id: "k",
            ciphertext: "c",
            encryption_algorithm: ENCRYPTION_ALGORITHM,
        })
        .expect("serializable");
        assert_eq!(
            body,
            serde_json::json!({
                "keyId": "k",
                "ciphertext": "c",
                "encryptionAlgorithm": "AES_256_GCM"
            })
        );
    }

    /// Tests against a real OCI Vault. Skipped by default; run with:
    ///
    /// ```text
    /// OCI_KMS_TEST_CRYPTO_ENDPOINT=https://<vault>-crypto.kms.<region>.oci.oraclecloud.com \
    /// OCI_KMS_TEST_KEY_ID=ocid1.key.oc1... \
    /// OCI_KMS_TEST_CLI_CIPHERTEXT=<output of `oci kms crypto encrypt`> \
    /// OCI_KMS_TEST_CLI_PLAINTEXT=<the plaintext that was encrypted> \
    /// cargo test -p oci_kms live -- --ignored
    /// ```
    ///
    /// Outside Kubernetes, credentials come from `~/.oci/config` (`OCI_CLI_PROFILE` to pick one).
    /// Inside a pod, the same tests exercise OKE Workload Identity instead.
    mod live {
        use std::time::{Duration, Instant};

        use super::*;

        fn env(name: &str) -> String {
            std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set for live tests"))
        }

        fn live_config() -> OciKmsConfig {
            OciKmsConfig {
                vault_crypto_endpoint: env("OCI_KMS_TEST_CRYPTO_ENDPOINT"),
                key_id: env("OCI_KMS_TEST_KEY_ID"),
            }
        }

        fn live_client() -> OciKmsClient {
            OciKmsClient::new(&live_config()).expect("client should build from the live config")
        }

        #[tokio::test]
        #[ignore = "calls a real OCI Vault"]
        async fn encrypt_then_decrypt_round_trips() {
            let client = live_client();

            let ciphertext = client.encrypt(b"s3cr3t!").await.expect("encrypt succeeds");
            assert_ne!(ciphertext.as_bytes(), b"s3cr3t!");
            // Printed so the OCI CLI can confirm it decrypts our ciphertext too.
            println!("OCI_KMS_LIVE_CIPHERTEXT={ciphertext}");

            let plaintext = client.decrypt(&ciphertext).await.expect("decrypt succeeds");
            assert_eq!(plaintext, b"s3cr3t!");
        }

        #[tokio::test]
        #[ignore = "calls a real OCI Vault"]
        async fn decrypts_ciphertext_made_by_the_oci_cli() {
            let plaintext = live_client()
                .decrypt(&env("OCI_KMS_TEST_CLI_CIPHERTEXT"))
                .await
                .expect("decrypt succeeds");
            assert_eq!(plaintext, env("OCI_KMS_TEST_CLI_PLAINTEXT").as_bytes());
        }

        #[tokio::test]
        #[ignore = "calls a real OCI Vault"]
        async fn generated_data_key_unwraps_to_its_plaintext() {
            let client = live_client();

            let data_key = client
                .generate_data_key()
                .await
                .expect("generate_data_key succeeds");
            assert_ne!(data_key.plaintext(), &[0; DATA_KEY_LENGTH]);

            let unwrapped = client
                .decrypt(data_key.ciphertext())
                .await
                .expect("decrypt succeeds");
            assert_eq!(unwrapped.as_slice(), data_key.plaintext());
        }

        #[tokio::test]
        #[ignore = "calls a real OCI Vault"]
        async fn decrypt_with_a_nonexistent_key_fails_without_retrying() {
            let mut config = live_config();
            config.key_id = format!("{}x", config.key_id);
            let client = OciKmsClient::new(&config).expect("client builds");

            let start = Instant::now();
            let result = client.decrypt(&env("OCI_KMS_TEST_CLI_CIPHERTEXT")).await;

            assert!(
                matches!(result, Err(OciKmsError::UnexpectedStatus { status, .. }) if status < 500),
                "expected a 4xx, got {result:?}"
            );
            println!("nonexistent key failed after {:?}", start.elapsed());
        }

        #[tokio::test]
        #[ignore = "needs OCI credentials; takes ~25s"]
        async fn unreachable_endpoint_times_out_and_retries() {
            let client = OciKmsClient::new(&OciKmsConfig {
                // TEST-NET-1 (RFC 5737): guaranteed unroutable, so every connect times out.
                vault_crypto_endpoint: "https://192.0.2.1".to_string(),
                key_id: env("OCI_KMS_TEST_KEY_ID"),
            })
            .expect("client builds");

            let start = Instant::now();
            let result = client.decrypt("ignored").await;
            let elapsed = start.elapsed();

            assert!(matches!(result, Err(OciKmsError::RequestFailed(_))));
            // Four attempts, each bounded by the 5s connect timeout.
            assert!(
                elapsed >= Duration::from_secs(15),
                "expected retries, failed after {elapsed:?}"
            );
            assert!(
                elapsed <= Duration::from_secs(40),
                "expected timeouts, took {elapsed:?}"
            );
            println!("unreachable endpoint failed after {elapsed:?}");
        }
    }
}
