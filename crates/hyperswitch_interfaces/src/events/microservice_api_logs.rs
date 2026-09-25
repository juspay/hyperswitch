//! Internal microservice API call logs interface

use router_env::RequestId;
use serde::Serialize;
use serde_json::json;
use time::OffsetDateTime;

/// API event for a call made to an internal microservice (e.g. the modular
/// payment methods service). Mirrors `ConnectorEvent` for outgoing connector
/// calls: one event per call, carrying the masked request, the outcome and
/// enough identifiers (merchant, profile, request id) to trace a failure.
#[derive(Debug, Serialize)]
pub struct MicroserviceEvent {
    tenant_id: common_utils::id_type::TenantId,
    /// Logical name of the microservice called (e.g. "payment_methods")
    service_name: String,
    /// The client operation executed (e.g. "CreatePaymentMethod")
    flow: String,
    /// Masked request body, when the operation carries one
    request: Option<String>,
    masked_response: Option<String>,
    error: Option<String>,
    url: String,
    method: String,
    merchant_id: Option<String>,
    profile_id: Option<String>,
    created_at: i128,
    /// Request id propagated to the microservice via the trace header
    pub request_id: String,
    latency: u128,
    status_code: Option<u16>,
}

impl MicroserviceEvent {
    /// Build a new microservice event for an in-flight call.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: common_utils::id_type::TenantId,
        service_name: String,
        flow: &str,
        request: Option<serde_json::Value>,
        url: String,
        method: common_utils::request::Method,
        merchant_id: Option<String>,
        profile_id: Option<String>,
        request_id: Option<&RequestId>,
    ) -> Self {
        Self {
            tenant_id,
            service_name,
            flow: flow
                .rsplit_once("::")
                .map(|(_, name)| name)
                .unwrap_or(flow)
                .to_string(),
            request: request.map(|value| value.to_string()),
            masked_response: None,
            error: None,
            url,
            method: method.to_string(),
            merchant_id,
            profile_id,
            created_at: OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000,
            request_id: request_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "NO_REQUEST_ID".to_string()),
            latency: 0,
            status_code: None,
        }
    }

    /// Record the latency of the completed call in milliseconds.
    pub fn set_latency(&mut self, latency: u128) {
        self.latency = latency;
    }

    /// Record the HTTP status code of the response.
    pub fn set_status_code(&mut self, code: u16) {
        self.status_code = Some(code);
    }

    /// Record a masked response body.
    pub fn set_masked_response<T: Serialize>(&mut self, response: &T) {
        match hyperswitch_masking::masked_serialize(response) {
            Ok(masked) => {
                self.masked_response = Some(masked.to_string());
            }
            Err(err) => self.set_error(json!({"error": err.to_string()})),
        }
    }

    /// Record the error the call failed with.
    pub fn set_error(&mut self, error: serde_json::Value) {
        self.error = Some(error.to_string());
    }

    /// Record the error as a plain string.
    ///
    /// Only for errors this crate produces itself (transport failures, decode failures).
    /// Upstream response bodies must go through [`Self::set_upstream_error`] instead.
    pub fn set_error_string(&mut self, error: String) {
        self.error = Some(error);
    }

    /// Record an upstream (non-2xx) response as a structured error.
    ///
    /// The raw body is never recorded: a microservice error response can echo request
    /// fields back, which for the payment methods service means raw card data. Only the
    /// diagnostic fields of a recognised error envelope are kept, so an unexpected key
    /// cannot carry a payload into the event pipe.
    pub fn set_upstream_error(&mut self, status_code: u16, body: &[u8]) {
        self.set_error(structure_upstream_error(status_code, body));
    }

    /// Returns the request id of the event.
    pub fn get_request_id(&self) -> &str {
        &self.request_id
    }

    /// Returns the merchant id of the event, when known.
    pub fn get_merchant_id(&self) -> Option<&str> {
        self.merchant_id.as_deref()
    }
}

/// Fields of an error envelope that describe the failure rather than the request that
/// caused it, and so are safe to record. Everything outside this list is dropped.
const SAFE_ERROR_FIELDS: [&str; 4] = ["type", "code", "message", "reason"];

/// Longest error field value recorded, in characters.
const ERROR_FIELD_MAX_CHARS: usize = 500;

/// Reduce an upstream error body to the diagnostic fields listed in [`SAFE_ERROR_FIELDS`].
///
/// A body that is not JSON, or whose JSON carries none of those fields, is recorded as a
/// shape-only description — never as its own bytes.
fn structure_upstream_error(status_code: u16, body: &[u8]) -> serde_json::Value {
    let details = match serde_json::from_slice::<serde_json::Value>(body) {
        Ok(parsed) => {
            // Hyperswitch-style envelopes nest the details under `error`.
            let envelope = parsed.get("error").unwrap_or(&parsed);
            let fields = SAFE_ERROR_FIELDS
                .iter()
                .filter_map(|field| {
                    let value = envelope.get(field)?.as_str()?;
                    let truncated: String = value.chars().take(ERROR_FIELD_MAX_CHARS).collect();
                    Some(((*field).to_string(), json!(truncated)))
                })
                .collect::<serde_json::Map<_, _>>();

            if fields.is_empty() {
                json!("unrecognised error envelope, not captured")
            } else {
                serde_json::Value::Object(fields)
            }
        }
        Err(_) => json!("non-json error body, not captured"),
    };

    json!({ "status_code": status_code, "error": details })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_diagnostic_fields_of_an_error_envelope() {
        let body = br#"{"error":{"type":"invalid_request","code":"IR_16","message":"Invalid card","card_number":"4111111111111111"}}"#;

        // The echoed `card_number` is dropped; only the diagnostic fields survive.
        assert_eq!(
            structure_upstream_error(400, body),
            json!({
                "status_code": 400,
                "error": {
                    "type": "invalid_request",
                    "code": "IR_16",
                    "message": "Invalid card",
                }
            })
        );
    }

    #[test]
    fn drops_a_flat_body_that_carries_no_diagnostic_fields() {
        let body = br#"{"card_number":"4111111111111111"}"#;

        let structured = structure_upstream_error(500, body);

        assert_eq!(
            structured,
            json!({
                "status_code": 500,
                "error": "unrecognised error envelope, not captured",
            })
        );
        assert!(!structured.to_string().contains("4111111111111111"));
    }

    #[test]
    fn drops_a_non_json_body() {
        let structured = structure_upstream_error(502, b"upstream said 4111111111111111");

        assert_eq!(
            structured,
            json!({
                "status_code": 502,
                "error": "non-json error body, not captured",
            })
        );
        assert!(!structured.to_string().contains("4111111111111111"));
    }

    #[test]
    fn truncates_a_long_diagnostic_field() {
        let message = "a".repeat(ERROR_FIELD_MAX_CHARS + 100);
        let body = json!({ "error": { "message": message } }).to_string();

        let recorded = structure_upstream_error(400, body.as_bytes());
        let recorded_message = recorded
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(|message| message.as_str());

        assert_eq!(
            recorded_message.map(|message| message.chars().count()),
            Some(ERROR_FIELD_MAX_CHARS)
        );
    }
}
