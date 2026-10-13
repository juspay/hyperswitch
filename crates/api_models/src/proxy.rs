use std::collections::HashMap;

pub use common_utils::proxy::{ProxyBody, ProxyMultipartPart};
use common_utils::{pii::SecretSerdeValue, request::Method};
use hyperswitch_masking::{ExposeInterface, Secret};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Headers(pub HashMap<String, String>);

impl hyperswitch_masking::SerializableSecret for Headers {}

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
    #[serde(default)]
    #[schema(value_type = serde_json::Value)]
    pub request_body: SecretSerdeValue,
    /// Explicit connector body format. Preferred over the legacy request_body field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<ProxyBody>)]
    pub body: Option<Secret<ProxyBody>>,
    /// Client certificate for the connector, independent of proxy authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub certificate: Option<Secret<String>>,
    /// Private key associated with the connector client certificate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub certificate_key: Option<Secret<String>>,
    /// Custom CA certificate for the connector connection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub ca_certificate: Option<Secret<String>>,
    /// The non-empty HTTP or HTTPS destination URL. Configured proxy bypass hosts are rejected.
    /// IP literals must be publicly routable.
    #[schema(value_type = String, min_length = 1, example = "https://api.example.com/endpoint")]
    pub destination_url: common_utils::outbound_url::SafeOutboundUrl,
    /// The headers that need to be forwarded
    #[schema(value_type = Object, example = r#"{ "key1": "value-1", "key2": "value-2" }"#)]
    pub headers: Secret<Headers>,
    /// The method that needs to be used for the request
    #[schema(value_type = Method, example = "POST")]
    pub method: Method,
    /// The vault token that is used to fetch sensitive data from the vault
    #[schema(value_type = String)]
    pub token: Secret<String>,
    /// The type of token that is used to fetch sensitive data from the vault
    #[schema(value_type = TokenType, example = "payment_method_id")]
    pub token_type: TokenType,
}

impl ProxyRequest {
    /// Interpret the body contract, preserving legacy GET/DELETE behavior. Those
    /// requests previously ignored request_body; a new explicit body is honored.
    pub fn connector_body(
        &self,
        content_type: Option<&str>,
    ) -> Result<ProxyBody, common_utils::proxy::ProxyBodyError> {
        match &self.body {
            Some(body) => Ok(body.clone().expose()),
            None if matches!(self.method, Method::Get | Method::Delete) => Ok(ProxyBody::None),
            None => ProxyBody::from_legacy(self.request_body.clone().expose(), content_type),
        }
    }
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
    #[schema(value_type = serde_json::Value)]
    pub response: SecretSerdeValue,
    /// Original connector body bytes. New clients must prefer this to `response`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub response_body_base64: Option<Secret<String>>,
    /// Connector headers preserving repeated names such as Set-Cookie.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Vec<(String, String)>>)]
    pub response_headers_multi: Option<Secret<Vec<(String, String)>>>,
    /// The status code of the response
    pub status_code: u16,
    /// The headers of the response
    #[schema(value_type = Object, example = r#"{ "key1": "value-1", "key2": "value-2" }"#)]
    pub response_headers: Secret<Headers>,
}

impl common_utils::events::ApiEventMetric for ProxyRequest {}
impl common_utils::events::ApiEventMetric for ProxyResponse {}

#[cfg(test)]
mod tests {
    use super::*;
    use hyperswitch_masking::PeekInterface;
    use serde_json::{json, Value};

    #[test]
    fn legacy_request_needs_no_new_fields() {
        let request: ProxyRequest = serde_json::from_value(json!({"request_body":{"card":"{{$card_number}}"},"destination_url":"https://connector.example/pay","headers":{"Content-Type":"application/json"},"method":"POST","token":"fixture-token","token_type":"payment_method_token"})).unwrap();
        assert!(request.body.is_none());
        assert!(request.certificate.is_none());
        assert_eq!(request.request_body.peek()["card"], "{{$card_number}}");
    }

    #[test]
    fn typed_request_can_omit_the_legacy_body() {
        let request: ProxyRequest = serde_json::from_value(json!({"body":{"type":"xml","data":"<card>{{$card_number}}</card>"},"destination_url":"https://connector.example/pay","headers":{"Content-Type":"text/xml; charset=UTF-8"},"method":"POST","token":"fixture-token","token_type":"payment_method_token"})).unwrap();
        assert!(matches!(
            request.body.as_ref().map(|body| body.peek()),
            Some(ProxyBody::Xml(_))
        ));
        assert_eq!(request.request_body.peek(), &Value::Null);
    }

    #[test]
    fn old_response_envelopes_still_deserialize() {
        let response: ProxyResponse = serde_json::from_value(json!({"response":{"approved":true},"status_code":200,"response_headers":{"Content-Type":"application/json"}})).unwrap();
        assert!(response.response_body_base64.is_none());
        assert!(response.response_headers_multi.is_none());
    }

    #[test]
    fn proxy_logging_masks_payloads_and_headers_without_changing_the_wire_format() {
        let pan = "4111111111111111";
        let encoded = "NDExMTExMTExMTExMTExMQ==";
        let request_wire = json!({
            "request_body": {"card": pan},
            "body": {"type":"xml", "data":format!("<card>{pan}</card>")},
            "destination_url":"https://connector.example/pay",
            "headers":{"Authorization":"Bearer fixture-secret"},
            "method":"POST",
            "token":"fixture-vault-token",
            "token_type":"payment_method_token"
        });
        let request: ProxyRequest = serde_json::from_value(request_wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(&request).unwrap(), request_wire);
        let request_log = hyperswitch_masking::masked_serialize(&request)
            .unwrap()
            .to_string();
        let request_debug = format!("{request:?}");
        for secret in [pan, "Bearer fixture-secret", "fixture-vault-token"] {
            assert!(!request_log.contains(secret));
            assert!(!request_debug.contains(secret));
        }

        let response_wire = json!({
            "response":{"echo":pan},
            "response_body_base64":encoded,
            "response_headers":{"Set-Cookie":"session=fixture-secret"},
            "response_headers_multi":[["Set-Cookie","session=fixture-secret"]],
            "status_code":422
        });
        let response: ProxyResponse = serde_json::from_value(response_wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(&response).unwrap(), response_wire);
        let response_log = hyperswitch_masking::masked_serialize(&response).unwrap();
        let response_debug = format!("{response:?}");
        assert_eq!(response_log["status_code"], 422);
        for secret in [pan, encoded, "session=fixture-secret"] {
            assert!(!response_log.to_string().contains(secret));
            assert!(!response_debug.contains(secret));
        }
    }

    #[test]
    fn legacy_get_and_delete_remain_bodyless_but_explicit_bodies_are_honored() {
        let mut request: ProxyRequest = serde_json::from_value(json!({"request_body":{},"destination_url":"https://connector.example/pay","headers":{},"method":"GET","token":"fixture-token","token_type":"payment_method_token"})).unwrap();
        for method in [Method::Get, Method::Delete] {
            request.method = method;
            assert!(matches!(
                request.connector_body(None).unwrap(),
                ProxyBody::None
            ));
        }
        request.body = Some(ProxyBody::Json("{\"explicit\":true}".into()).into());
        assert!(matches!(
            request.connector_body(None).unwrap(),
            ProxyBody::Json(_)
        ));
    }

    #[test]
    fn typed_proxy_bodies_keep_destination_validation() {
        for body in [
            json!({"type":"json","data":"{}"}),
            json!({"type":"xml","data":"<card/>"}),
            json!({"type":"form_url_encoded","data":"card=fixture"}),
            json!({"type":"text","data":"fixture"}),
            json!({"type":"multipart","data":[]}),
            json!({"type":"raw_bytes","data":"AA=="}),
            json!({"type":"none"}),
        ] {
            let mut request = json!({
                "body": body,
                "destination_url": "https://connector.example/pay",
                "headers": {},
                "method": "POST",
                "token": "fixture-token",
                "token_type": "payment_method_token"
            });
            let parsed: ProxyRequest = serde_json::from_value(request.clone()).unwrap();
            assert_eq!(
                parsed.destination_url.get_string_repr(),
                "https://connector.example/pay"
            );
            for destination in [
                "",
                "file:///etc/passwd",
                "http://127.0.0.1/pay",
                "http://10.0.0.1/pay",
                "http://169.254.169.254/latest/meta-data/",
                "http://[::1]/pay",
            ] {
                request["destination_url"] = json!(destination);
                assert!(serde_json::from_value::<ProxyRequest>(request.clone()).is_err());
            }
        }
    }

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
