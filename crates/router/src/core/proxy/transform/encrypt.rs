//! Encrypt steps: encrypt a string field of the JSON body in place.

use api_models::proxy::{Encoding, EncryptScheme};
use base64::Engine;
use common_utils::{consts::BASE64_ENGINE, crypto};
use rsa::{pkcs8::DecodePublicKey, RsaPublicKey};
use serde_json::Value;

use super::{encode, Body, Payload, Step, TransformError};

/// An encryption algorithm.
pub trait Encryption {
    /// Check the key before the vault is read.
    fn validate_key(&self, key: &[u8]) -> Result<(), TransformError>;
    /// Encrypt the plaintext with the key.
    fn encrypt(&self, key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, TransformError>;
}

impl Encryption for EncryptScheme {
    fn validate_key(&self, key: &[u8]) -> Result<(), TransformError> {
        match self {
            Self::RsaOaepSha256 => RsaPublicKey::from_public_key_der(key)
                .map(|_| ())
                .map_err(|_| TransformError::InvalidKey),
        }
    }

    fn encrypt(&self, key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, TransformError> {
        match self {
            Self::RsaOaepSha256 => {
                crypto::encrypt_rsa_oaep_sha256(key, plaintext).map_err(|_| TransformError::Failed)
            }
        }
    }
}

/// Replaces the string at `field` with its encoded ciphertext.
#[derive(Debug)]
pub struct EncryptStep {
    field: String,
    scheme: EncryptScheme,
    key: Vec<u8>,
    encoding: Encoding,
}

impl EncryptStep {
    /// The field must be a string in the body template, and the key is base64-encoded.
    pub fn new(
        field: &str,
        scheme: EncryptScheme,
        key: &str,
        encoding: Encoding,
        template: &Value,
    ) -> Result<Self, TransformError> {
        template
            .pointer(field)
            .filter(|value| value.is_string())
            .ok_or(TransformError::InvalidField)?;
        let key = BASE64_ENGINE
            .decode(key)
            .map_err(|_| TransformError::InvalidKey)?;
        scheme.validate_key(&key)?;
        Ok(Self {
            field: field.to_owned(),
            scheme,
            key,
            encoding,
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
        let ciphertext = self.scheme.encrypt(&self.key, plaintext.as_bytes())?;
        *field = Value::String(encode(self.encoding, &ciphertext));
        Ok(Payload {
            body: Body::Json(json),
            headers: payload.headers,
        })
    }
}
