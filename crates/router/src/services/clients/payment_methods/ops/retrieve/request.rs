// TODO: replace dummy request types with real v1/v2 models.
#[derive(Clone, Debug)]
pub struct RetrieveV1Request {
    pub payment_method_id: String,
}

#[derive(Clone, Debug)]
pub struct RetrieveV2Request {
    pub body: Option<serde_json::Value>,
}
