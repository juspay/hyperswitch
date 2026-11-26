//! Transformation instructions for the vault proxy, shared by the proxy API and the injector so
//! callers build them with the same types the proxy accepts.

use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Steps run in order: encrypt steps, then exactly one serialize step, then sign steps.
#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct ProxyTransformation {
    /// The steps to run on the body
    pub steps: Vec<ProxyTransformInstruction>,
}

/// One transformation step
#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ProxyTransformInstruction {
    /// Encrypt a string field of the JSON body and replace it with the encoded ciphertext
    Encrypt {
        /// JSON Pointer to the string field to encrypt, such as
        /// `/source/encryptionData/encryptionBlock`. Its value after vault substitution is the
        /// plaintext; it is replaced in place by the encoded ciphertext.
        field: String,
        /// The encryption algorithm
        scheme: ProxyEncryptScheme,
        /// Base64-encoded DER public key in SubjectPublicKeyInfo (SPKI) format
        key: String,
        /// The encoding of the ciphertext written to the field
        encoding: ProxyEncoding,
    },
    /// Serialize the JSON body to the bytes that are signed and sent
    Serialize {
        /// The serialization format
        format: ProxySerializeFormat,
    },
    /// Sign a message built from request headers and the body bytes, and write the encoded
    /// signature to a header
    Sign {
        /// The parts of the signed message, concatenated in order
        message: Vec<ProxyMessagePart>,
        /// The signing algorithm
        scheme: ProxySignScheme,
        /// The signing secret
        #[schema(value_type = String)]
        key: Secret<String>,
        /// The encoding of the signature
        encoding: ProxyEncoding,
        /// The header the signature is written to; it must not already be in the request
        header: String,
    },
}

/// Encryption algorithms
#[derive(Debug, ToSchema, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyEncryptScheme {
    /// RSA-OAEP with SHA-256
    RsaOaepSha256,
}

/// Signing algorithms
#[derive(Debug, ToSchema, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxySignScheme {
    /// HMAC with SHA-256
    HmacSha256,
}

/// Serialization formats
#[derive(Debug, ToSchema, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxySerializeFormat {
    /// JSON
    Json,
}

/// Encodings for ciphertexts and signatures
#[derive(Debug, ToSchema, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyEncoding {
    /// Standard base64 with padding
    Base64,
}

/// A part of a signed message: `{"header": "Api-Key"}` or `"body"`
#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyMessagePart {
    /// Name of a request header whose value is signed
    Header(String),
    /// The serialized body bytes
    Body,
}
