//! Connector API logs interface

use common_utils::request::Method;
use router_env::RequestId;
use serde::Serialize;
use serde_json::json;
use time::OffsetDateTime;

use crate::consts::CONNECTOR_EVENT_SOURCE;

/// struct ConnectorEvent
#[derive(Debug, Serialize)]
pub struct ConnectorEvent {
    tenant_id: common_utils::id_type::TenantId,
    connector_name: String,
    flow: String,
    request: String,
    masked_response: Option<String>,
    error: Option<String>,
    url: String,
    method: String,
    merchant_id: common_utils::id_type::MerchantId,
    created_at: i128,
    /// Connector Event Request ID
    pub request_id: String,
    latency: u128,
    status_code: u16,
    /// Service that produced this event (always `hyperswitch` here).
    source: &'static str,
    /// Primary (real) execution or shadow mirror — the two-state event projection of the
    /// routing `ExecutionMode` (see `common_enums::EventExecutionMode`).
    execution_mode: common_enums::EventExecutionMode,
    #[serde(flatten)]
    connector_event_type: common_utils::events::ConnectorEventsType,
}

impl ConnectorEvent {
    /// fn new ConnectorEvent
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: common_utils::id_type::TenantId,
        connector_name: String,
        flow: &str,
        request: serde_json::Value,
        url: String,
        method: Method,
        payment_id: String,
        merchant_id: common_utils::id_type::MerchantId,
        request_id: Option<&RequestId>,
        latency: u128,
        refund_id: Option<String>,
        dispute_id: Option<String>,
        payout_id: Option<String>,
        status_code: u16,
        execution_mode: common_enums::EventExecutionMode,
    ) -> Self {
        let connector_event_type = common_utils::events::ConnectorEventsType::new(
            payment_id, refund_id, payout_id, dispute_id,
        );
        Self {
            tenant_id,
            connector_name,
            flow: flow
                .rsplit_once("::")
                .map(|(_, s)| s)
                .unwrap_or(flow)
                .to_string(),
            request: request.to_string(),
            masked_response: None,
            error: None,
            url,
            method: method.to_string(),
            merchant_id,
            created_at: OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000,
            request_id: request_id
                .map(|i| i.to_string())
                .unwrap_or("NO_REQUEST_ID".to_string()),
            latency,
            status_code,
            source: CONNECTOR_EVENT_SOURCE,
            execution_mode,
            connector_event_type,
        }
    }

    /// fn set_response_body
    pub fn set_response_body<T: Serialize>(&mut self, response: &T) {
        match hyperswitch_masking::masked_serialize(response) {
            Ok(masked) => {
                self.masked_response = Some(masked.to_string());
            }
            Err(er) => self.set_error(json!({"error": er.to_string()})),
        }
    }

    /// fn set_error_response_body
    pub fn set_error_response_body<T: Serialize>(&mut self, response: &T) {
        match hyperswitch_masking::masked_serialize(response) {
            Ok(masked) => {
                self.error = Some(masked.to_string());
            }
            Err(er) => self.set_error(json!({"error": er.to_string()})),
        }
    }

    /// fn set_error
    pub fn set_error(&mut self, error: serde_json::Value) {
        self.error = Some(error.to_string());
    }
}

/// A Hyperswitch -> Unified Connector Service (UCS) gRPC call, recorded as its own event.
///
/// Carries the gRPC envelope rather than a connector HTTP body, so it maps to its own
/// `ucs_api_events` stream. Same wire schema as [`ConnectorEvent`]: a transparent newtype
/// reusing its fields and setters, but a distinct `EventType`.
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct UcsApiEvent(ConnectorEvent);

impl UcsApiEvent {
    /// fn new UcsApiEvent
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: common_utils::id_type::TenantId,
        connector_name: String,
        flow: &str,
        request: serde_json::Value,
        url: String,
        method: Method,
        payment_id: String,
        merchant_id: common_utils::id_type::MerchantId,
        request_id: Option<&RequestId>,
        latency: u128,
        refund_id: Option<String>,
        dispute_id: Option<String>,
        payout_id: Option<String>,
        status_code: u16,
        execution_mode: common_enums::EventExecutionMode,
    ) -> Self {
        Self(ConnectorEvent::new(
            tenant_id,
            connector_name,
            flow,
            request,
            url,
            method,
            payment_id,
            merchant_id,
            request_id,
            latency,
            refund_id,
            dispute_id,
            payout_id,
            status_code,
            execution_mode,
        ))
    }

    /// Request ID of the underlying event (used as the Kafka partition key).
    pub fn request_id(&self) -> &str {
        &self.0.request_id
    }

    /// fn set_response_body
    pub fn set_response_body<T: Serialize>(&mut self, response: &T) {
        self.0.set_response_body(response);
    }

    /// fn set_error_response_body
    pub fn set_error_response_body<T: Serialize>(&mut self, response: &T) {
        self.0.set_error_response_body(response);
    }
}
