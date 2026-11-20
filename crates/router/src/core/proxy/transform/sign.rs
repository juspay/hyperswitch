//! Sign steps: sign a message built from request headers and the body bytes, and write the
//! encoded signature to a header. The body bytes are not changed.

use std::collections::HashMap;

use api_models::proxy::{Encoding, MessagePart, SignScheme};
use common_utils::{
    crypto::{self, SignMessage},
    fp_utils::when,
};
use hyperswitch_masking::{Mask, PeekInterface, Secret};

use super::{encode, header_value, Body, Payload, Step, TransformError};

/// A signing algorithm.
pub trait Signing {
    /// Sign the message with the key.
    fn sign(&self, key: &[u8], message: &[u8]) -> Result<Vec<u8>, TransformError>;
}

impl Signing for SignScheme {
    fn sign(&self, key: &[u8], message: &[u8]) -> Result<Vec<u8>, TransformError> {
        match self {
            Self::HmacSha256 => crypto::HmacSha256
                .sign_message(key, message)
                .map_err(|_| TransformError::Failed),
        }
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
    scheme: SignScheme,
    key: Secret<String>,
    encoding: Encoding,
    header: String,
}

impl SignStep {
    /// The message must not be empty, every header in it must be in the request and sent as it
    /// is, and the key must not be empty.
    pub fn new(
        message: &[MessagePart],
        scheme: SignScheme,
        key: &Secret<String>,
        encoding: Encoding,
        header: &str,
        headers: &HashMap<String, String>,
    ) -> Result<Self, TransformError> {
        when(message.is_empty(), || Err(TransformError::InvalidMessage))?;
        when(key.peek().is_empty(), || Err(TransformError::InvalidKey))?;
        message
            .iter()
            .map(|part| match part {
                MessagePart::Header(name) => {
                    when(
                        UNSIGNABLE_HEADERS
                            .iter()
                            .any(|unsignable| name.eq_ignore_ascii_case(unsignable)),
                        || Err(TransformError::InvalidMessage),
                    )?;
                    header_value(headers, name)
                        .map(|value| Part::Text(Secret::new(value.to_owned())))
                }
                MessagePart::Body => Ok(Part::Body),
            })
            .collect::<Result<_, _>>()
            .map(|message| Self {
                message,
                scheme,
                key: key.clone(),
                encoding,
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
        let signature = self.scheme.sign(self.key.peek().as_bytes(), &message)?;
        let headers = payload
            .headers
            .into_iter()
            .chain([(
                self.header.clone(),
                encode(self.encoding, &signature).into_masked(),
            )])
            .collect();
        Ok(Payload {
            body: payload.body,
            headers,
        })
    }
}
