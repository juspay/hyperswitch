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
    pub fn set_error_string(&mut self, error: String) {
        self.error = Some(error);
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
