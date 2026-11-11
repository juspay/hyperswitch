use std::collections::HashMap;

use common_utils::request::Method;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Headers(pub HashMap<String, String>);

impl Headers {
    pub fn as_map(&self) -> &HashMap<String, String> {
        &self.0
    }

    pub fn from_header_map(headers: Option<&HeaderMap>) -> Self {
        headers
            .map(|h| {
                let map = h
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                Self(map)
            })
            .unwrap_or_else(|| Self(HashMap::new()))
    }
}

#[derive(Debug, ToSchema, Clone, Deserialize, Serialize)]
pub struct ProxyRequest {
    /// The request body that needs to be forwarded
    pub request_body: Value,
    /// The non-empty HTTP or HTTPS destination URL. Configured proxy bypass hosts are rejected.
    /// IP literals must be publicly routable.
    #[schema(value_type = String, min_length = 1, example = "https://api.example.com/endpoint")]
    pub destination_url: common_utils::outbound_url::SafeOutboundUrl,
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
    VolatilePaymentMethodId,
    PaymentMethodToken,
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

#[cfg(test)]
mod tests {
    use super::ProxyRequest;

    #[test]
    fn proxy_destination_cannot_be_empty() {
        let mut request = serde_json::json!({
            "request_body": {},
            "destination_url": "https://merchant.example.com/hook",
            "headers": {},
            "method": "GET",
            "token": "pm_example",
            "token_type": "payment_method_id"
        });
        assert!(serde_json::from_value::<ProxyRequest>(request.clone()).is_ok());
        request
            .as_object_mut()
            .expect("proxy request")
            .insert("destination_url".to_string(), serde_json::json!(""));
        assert!(serde_json::from_value::<ProxyRequest>(request).is_err());
    }
}
