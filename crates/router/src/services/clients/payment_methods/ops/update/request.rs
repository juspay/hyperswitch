// TODO: replace dummy request types with real v1/v2 models.
#[derive(Clone, Debug)]
pub struct UpdateV1Request {
    pub payment_method_id: String,
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug)]
pub struct UpdateV2Request {
    pub body: Option<serde_json::Value>,
}
