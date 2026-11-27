//! Serialize steps: turn the JSON body into the bytes that are signed and sent.

use api_models::proxy::ProxySerializeFormat;
use serde_json::Value;

use super::{Body, Payload, Step, TransformError};

/// A serialization format.
pub trait Serialization: std::fmt::Debug + Send + Sync {
    /// Serialize the body to bytes.
    fn serialize(&self, body: &Value) -> Result<Vec<u8>, TransformError>;
}

/// Compact JSON.
#[derive(Debug)]
struct Json;

impl Serialization for Json {
    fn serialize(&self, body: &Value) -> Result<Vec<u8>, TransformError> {
        serde_json::to_vec(body).map_err(|_| TransformError::Failed)
    }
}

/// The serializer a format names.
fn serializer(format: ProxySerializeFormat) -> Box<dyn Serialization> {
    match format {
        ProxySerializeFormat::Json => Box::new(Json),
    }
}

/// Serializes the JSON body.
#[derive(Debug)]
pub struct SerializeStep {
    format: Box<dyn Serialization>,
}

impl SerializeStep {
    /// A step that serializes with `format`.
    pub fn new(format: ProxySerializeFormat) -> Self {
        Self {
            format: serializer(format),
        }
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
