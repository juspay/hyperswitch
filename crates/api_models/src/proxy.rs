use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct ProxyRequest {
    /// The request body that needs to be forwarded
    pub req_body: Value,
    /// The destination URL where the request needs to be forwarded
    #[schema(example = "https://api.example.com/endpoint")]
    pub destination_url: String,
    /// The headers that need to be forwarded
    pub headers: Value,
    /// The vault token that is used to fetch sensitive data from the vault
    pub token: String,
    /// The type of token that is used to fetch sensitive data from the vault
    pub token_type: TokenType
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub enum TokenType {
    TokenizationId,
    PaymentMethodId
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct ProxyResponse {
    /// The response received from the destination
    pub response: Value,
    pub status_code: u16,
    pub response_headers: Value,
}

impl common_utils::events::ApiEventMetric for ProxyRequest {}
impl common_utils::events::ApiEventMetric for ProxyResponse {}