//! Serialize steps: turn the JSON body into the bytes that are signed and sent.

use api_models::proxy::SerializeFormat;
use serde_json::Value;

use super::{Body, Payload, Step, TransformError};

/// A serialization format.
pub trait Serialization {
    /// Serialize the body to bytes.
    fn serialize(&self, body: &Value) -> Result<Vec<u8>, TransformError>;
}

impl Serialization for SerializeFormat {
    fn serialize(&self, body: &Value) -> Result<Vec<u8>, TransformError> {
        match self {
            Self::Json => serde_json::to_vec(body).map_err(|_| TransformError::Failed),
        }
    }
}

/// Serializes the JSON body.
#[derive(Debug)]
pub struct SerializeStep {
    format: SerializeFormat,
}

impl SerializeStep {
    /// A step that serializes with `format`.
    pub fn new(format: SerializeFormat) -> Self {
        Self { format }
    }
}

impl Step for SerializeStep {
    fn execute(&self, payload: Payload) -> Result<Payload, TransformError> {
        match payload.body {
            Body::Json(json) => Ok(Payload {
                body: Body::Bytes(self.format.serialize(&json)?),
                headers: payload.headers,
            }),
            Body::Bytes(_) => Err(TransformError::InvalidOrder),
        }
    }
}
