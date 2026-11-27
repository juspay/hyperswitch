//! Encrypt steps: encrypt a string field of the JSON body in place.

use api_models::proxy::{ProxyEncoding, ProxyEncryptScheme};
use base64::Engine;
use common_utils::{consts::BASE64_ENGINE, crypto};
use rsa::{pkcs8::DecodePublicKey, traits::PublicKeyParts, RsaPublicKey};
use serde_json::Value;

use super::{Body, EncodedAlgorithm, Payload, Step, TransformError};

/// The smallest RSA key accepted: 2048 bits, the minimum PCI DSS treats as strong cryptography.
const MIN_RSA_KEY_BYTES: usize = 256;

/// An encryption algorithm with its key, checked when the algorithm is built.
pub trait Encryption: std::fmt::Debug + Send + Sync {
    /// Encrypt the plaintext.
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, TransformError>;
}

/// RSA-OAEP with SHA-256 for the hash and MGF1, and no label.
#[derive(Debug)]
struct RsaOaepSha256 {
    /// DER public key in SubjectPublicKeyInfo format
    public_key: Vec<u8>,
}

impl RsaOaepSha256 {
    /// The key must be an RSA public key of at least 2048 bits.
    fn new(public_key: Vec<u8>) -> Result<Self, TransformError> {
        RsaPublicKey::from_public_key_der(&public_key)
            .ok()
            .filter(|parsed| parsed.size() >= MIN_RSA_KEY_BYTES)
            .ok_or(TransformError::InvalidKey)?;
        Ok(Self { public_key })
    }
}

impl Encryption for RsaOaepSha256 {
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, TransformError> {
        crypto::encrypt_rsa_oaep_sha256(&self.public_key, plaintext)
            .map_err(|_| TransformError::Failed)
    }
}

/// The algorithm a scheme names, built with its key.
fn algorithm(
    scheme: ProxyEncryptScheme,
    key: Vec<u8>,
) -> Result<Box<dyn Encryption>, TransformError> {
    match scheme {
        ProxyEncryptScheme::RsaOaepSha256 => {
            RsaOaepSha256::new(key).map(|rsa| -> Box<dyn Encryption> { Box::new(rsa) })
        }
    }
}

impl EncodedAlgorithm<Box<dyn Encryption>> {
    /// Encrypt the plaintext and encode the ciphertext.
    fn encrypt(&self, plaintext: &[u8]) -> Result<String, TransformError> {
        self.algorithm
            .encrypt(plaintext)
            .map(|ciphertext| self.encode(&ciphertext))
    }
}

/// Replaces the string at `field` with its encoded ciphertext.
#[derive(Debug)]
pub struct EncryptStep {
    field: String,
    scheme: EncodedAlgorithm<Box<dyn Encryption>>,
}

impl EncryptStep {
    /// The field must be a string in the body template, and the key is base64-encoded and valid
    /// for the scheme.
    pub fn new(
        field: &str,
        scheme: ProxyEncryptScheme,
        key: &str,
        encoding: ProxyEncoding,
        template: &Value,
    ) -> Result<Self, TransformError> {
        template
            .pointer(field)
            .filter(|value| value.is_string())
            .ok_or(TransformError::InvalidField)?;
        let key = BASE64_ENGINE
            .decode(key)
            .map_err(|_| TransformError::InvalidKey)?;
        Ok(Self {
            field: field.to_owned(),
            scheme: EncodedAlgorithm {
                algorithm: algorithm(scheme, key)?,
                encoding,
            },
        })
    }
}

impl Step for EncryptStep {
    fn execute(&self, payload: Payload) -> Result<Payload, TransformError> {
        let mut json = match payload.body {
            Body::Json(json) => Ok(json),
            Body::Bytes(_) => Err(TransformError::InvalidOrder),
        }?;
        let field = json
            .pointer_mut(&self.field)
            .ok_or(TransformError::InvalidField)?;
        let plaintext = field.as_str().ok_or(TransformError::InvalidField)?;
        let ciphertext = self.scheme.encrypt(plaintext.as_bytes())?;
        *field = Value::String(ciphertext);
        Ok(Payload {
            body: Body::Json(json),
            headers: payload.headers,
        })
    }
}
