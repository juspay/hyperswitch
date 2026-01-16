// TODO: replace dummy response types with real v1/v2 models.
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct RetrieveV2Response {
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct RetrieveV1Response {
    pub payment_method_id: String,
    pub deleted: Option<bool>,
}
