//! Sign steps: sign a message built from request headers and the body bytes, and write the
//! encoded signature to a header. The body bytes are not changed.

use api_models::proxy::{Headers, ProxyEncoding, ProxyMessagePart, ProxySignScheme};
use common_utils::{
    crypto::{self, SignMessage},
    fp_utils::when,
};
use hyperswitch_masking::{Mask, PeekInterface, Secret};

use super::{Body, EncodedAlgorithm, Payload, Step, TransformError};

/// A signing algorithm with its key, checked when the algorithm is built.
pub trait Signing: std::fmt::Debug + Send + Sync {
    /// Sign the message.
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, TransformError>;
}

/// HMAC with SHA-256.
#[derive(Debug)]
struct HmacSha256 {
    key: Secret<Vec<u8>>,
}

impl HmacSha256 {
    /// The key must not be empty.
    fn new(key: &Secret<String>) -> Result<Self, TransformError> {
        when(key.peek().is_empty(), || Err(TransformError::InvalidKey))?;
        Ok(Self {
            key: Secret::new(key.peek().as_bytes().to_vec()),
        })
    }
}

impl Signing for HmacSha256 {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, TransformError> {
        crypto::HmacSha256
            .sign_message(self.key.peek(), message)
            .map_err(|_| TransformError::Failed)
    }
}

/// The algorithm a scheme names, built with its key.
fn algorithm(
    scheme: ProxySignScheme,
    key: &Secret<String>,
) -> Result<Box<dyn Signing>, TransformError> {
    match scheme {
        ProxySignScheme::HmacSha256 => {
            HmacSha256::new(key).map(|hmac| -> Box<dyn Signing> { Box::new(hmac) })
        }
    }
}

impl EncodedAlgorithm<Box<dyn Signing>> {
    /// Sign the message and encode the signature.
    fn sign(&self, message: &[u8]) -> Result<String, TransformError> {
        self.algorithm
            .sign(message)
            .map(|signature| self.encode(&signature))
    }
}

/// Headers the proxy drops without setting them again, so a signature over their values would
/// not match what is sent.
const UNSIGNABLE_HEADERS: [&str; 2] = ["content-length", "transfer-encoding"];

/// A part of the signed message. Header values are read before the vault is read, as they do
/// not change.
#[derive(Debug)]
enum Part {
    Text(Secret<String>),
    Body,
}

/// Signs the concatenated message parts and writes the signature to `header`.
#[derive(Debug)]
pub struct SignStep {
    message: Vec<Part>,
    scheme: EncodedAlgorithm<Box<dyn Signing>>,
    header: String,
}

impl SignStep {
    /// The message must not be empty, every header in it must be in the request and sent as it
    /// is, and the key must be valid for the scheme.
    pub fn new(
        message: &[ProxyMessagePart],
        scheme: ProxySignScheme,
        key: &Secret<String>,
        encoding: ProxyEncoding,
        header: &str,
        headers: &Headers,
    ) -> Result<Self, TransformError> {
        when(message.is_empty(), || Err(TransformError::InvalidMessage))?;
        let scheme = EncodedAlgorithm {
            algorithm: algorithm(scheme, key)?,
            encoding,
        };
        message
            .iter()
            .map(|part| match part {
                ProxyMessagePart::Header(name) => {
                    when(
                        UNSIGNABLE_HEADERS
                            .iter()
                            .any(|unsignable| name.eq_ignore_ascii_case(unsignable)),
                        || Err(TransformError::InvalidMessage),
                    )?;
                    headers
                        .get(name)
                        .map_err(|_| TransformError::ConflictingHeader)?
                        .map(|value| Part::Text(Secret::new(value.to_owned())))
                        .ok_or(TransformError::MissingHeader)
                }
                ProxyMessagePart::Body => Ok(Part::Body),
            })
            .collect::<Result<_, _>>()
            .map(|message| Self {
                message,
                scheme,
                header: header.to_owned(),
            })
    }
}

impl Step for SignStep {
    fn execute(&self, payload: Payload) -> Result<Payload, TransformError> {
        let bytes = match &payload.body {
            Body::Bytes(bytes) => Ok(bytes),
            Body::Json(_) => Err(TransformError::InvalidOrder),
        }?;
        let message: Vec<u8> = self
            .message
            .iter()
            .flat_map(|part| match part {
                Part::Text(text) => text.peek().as_bytes(),
                Part::Body => bytes.as_slice(),
            })
            .copied()
            .collect();
        let signature = self.scheme.sign(&message)?;
        let headers = payload
            .headers
            .into_iter()
            .chain([(self.header.clone(), signature.into_masked())])
            .collect();
        Ok(Payload {
            body: payload.body,
            headers,
        })
    }
}
