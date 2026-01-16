// TODO: replace dummy request types with real v1/v2 models.
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct CreateV1Request {
    pub payload: Value,
}

#[derive(Clone, Debug)]
pub struct CreateV2Request {
    pub body: Option<Value>,
}
