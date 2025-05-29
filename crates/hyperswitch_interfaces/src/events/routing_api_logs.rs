//! Routing API logs interface

use api_models::routing::RoutableConnectorChoice;
use common_utils::request::Method;
use router_env::tracing_actix_web::RequestId;
use serde::Serialize;
use serde_json::json;
use time::OffsetDateTime;

/// RoutingEngine enum
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingEngine {
    /// Dynamo for routing
    IntelligentRouter,
    /// Decision engine for routing
    DecisionEngine,
}

#[derive(Debug, Serialize)]
/// RoutingEvent type
pub struct RoutingEvent {
    tenant_id: common_utils::id_type::TenantId,
    routable_connectors: Vec<RoutableConnectorChoice>,
    flow: String,
    request: String,
    masked_response: Option<String>,
    error: Option<String>,
    url: String,
    method: String,
    payment_id: String,
    profile_id: common_utils::id_type::ProfileId,
    created_at: i128,
    status_code: Option<u16>,
    request_id: String,
    routing_engine: RoutingEngine,
}

impl RoutingEvent {
    /// fn new RoutingEvent
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: common_utils::id_type::TenantId,
        routable_connectors: Vec<RoutableConnectorChoice>,
        flow: &str,
        request: serde_json::Value,
        url: String,
        method: Method,
        payment_id: String,
        profile_id: common_utils::id_type::ProfileId,
        request_id: Option<RequestId>,
        routing_engine: RoutingEngine,
    ) -> Self {
        Self {
            tenant_id,
            routable_connectors,
            flow: flow.to_string(),
            request: request.to_string(),
            masked_response: None,
            error: None,
            url,
            method: method.to_string(),
            payment_id,
            profile_id,
            created_at: OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000,
            status_code: None,
            request_id: request_id
                .map(|i| i.as_hyphenated().to_string())
                .unwrap_or("NO_REQUEST_ID".to_string()),
            routing_engine,
        }
    }

    /// fn set_response_body
    pub fn set_response_body<T: Serialize>(&mut self, response: &T) {
        match masking::masked_serialize(response) {
            Ok(masked) => {
                self.masked_response = Some(masked.to_string());
            }
            Err(er) => self.set_error(json!({"error": er.to_string()})),
        }
    }

    /// fn set_error_response_body
    pub fn set_error_response_body<T: Serialize>(&mut self, response: &T) {
        match masking::masked_serialize(response) {
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

    /// set response status code
    pub fn set_status_code(&mut self, code: u16) {
        self.status_code = Some(code);
    }

    /// set response status code
    pub fn set_routable_connectors(&mut self, connectors: Vec<RoutableConnectorChoice>) {
        self.routable_connectors = connectors;
    }

    /// Returns the request ID of the event.
    pub fn get_request_id(&self) -> &str {
        &self.request_id
    }
}
