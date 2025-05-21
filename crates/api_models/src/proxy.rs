use std::collections::HashMap;

use common_utils::request::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Headers(pub HashMap<String, String>);

impl Headers {
    pub fn as_map(&self) -> &HashMap<String, String> {
        &self.0
    }
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct ProxyRequest {
    /// The request body that needs to be forwarded
    pub request_body: Value,
    /// The destination URL where the request needs to be forwarded
    #[schema(value_type = String, example = "https://api.example.com/endpoint")]
    pub destination_url: url::Url,
    /// The headers that need to be forwarded
    #[schema(value_type = Object, example = r#"{ "key1": "value-1", "key2": "value-2" }"#)]
    pub headers: Headers,
    /// The method that needs to be used for the request
    #[schema(value_type = Method, example = "Post")]
    pub method: Method,
    /// The vault token that is used to fetch sensitive data from the vault
    pub token: String,
    /// The type of token that is used to fetch sensitive data from the vault
    #[schema(value_type = TokenType, example = "payment_method_id")]
    pub token_type: TokenType,
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    TokenizationId,
    PaymentMethodId,
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct ProxyResponse {
    /// The response received from the destination
    pub response: Value,
    /// The status code of the response
    pub status_code: u16,
    /// The headers of the response
    #[schema(value_type = Object, example = r#"{ "key1": "value-1", "key2": "value-2" }"#)]
    pub response_headers: Headers,
}

impl common_utils::events::ApiEventMetric for ProxyRequest {}
impl common_utils::events::ApiEventMetric for ProxyResponse {}
