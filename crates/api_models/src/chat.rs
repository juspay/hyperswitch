#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ChatMessageQueryParam {
    pub message: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ChatResponse {
    pub message: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ChatDataType {
    pub output: Option<serde_json::Value>,
}
