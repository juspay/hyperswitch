#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ExternalFeeEstimateRequest {
    pub payload: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ExternalFeeEstimatePayload {
    pub payload: serde_json::Value,
}

#[derive(serde::Serialize, Debug)]
#[serde(untagged)]
pub enum ExternalFeeEstimateResponse {
    Hypersense { response: serde_json::Value },
}
