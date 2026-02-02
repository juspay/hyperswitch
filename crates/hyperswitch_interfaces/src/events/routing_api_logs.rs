//! Routing API logs interface

use std::fmt;

use api_models::routing::RoutableConnectorChoice;
use common_utils::request::Method;
use router_env::RequestId;
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

/// Method type enum
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiMethod {
    /// grpc call
    Grpc,
    /// Rest call
    Rest(Method),
}

impl fmt::Display for ApiMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grpc => write!(f, "Grpc"),
            Self::Rest(method) => write!(f, "Rest ({method})"),
        }
    }
}

#[derive(Debug, Serialize)]
/// RoutingEvent type
pub struct RoutingEvent {
    tenant_id: common_utils::id_type::TenantId,
    routable_connectors: String,
    payment_connector: Option<String>,
    flow: String,
    request: String,
    response: Option<String>,
    error: Option<String>,
    url: String,
    method: String,
    payment_id: String,
    profile_id: common_utils::id_type::ProfileId,
    merchant_id: common_utils::id_type::MerchantId,
    created_at: i128,
    status_code: Option<u16>,
    request_id: String,
    routing_engine: RoutingEngine,
    routing_approach: Option<String>,
}

impl RoutingEvent {
    /// fn new RoutingEvent
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: common_utils::id_type::TenantId,
        routable_connectors: String,
        flow: &str,
        request: serde_json::Value,
        url: String,
        method: ApiMethod,
        payment_id: String,
        profile_id: common_utils::id_type::ProfileId,
        merchant_id: common_utils::id_type::MerchantId,
        request_id: Option<RequestId>,
        routing_engine: RoutingEngine,
    ) -> Self {
        Self {
            tenant_id,
            routable_connectors,
            flow: flow.to_string(),
            request: request.to_string(),
            response: None,
            error: None,
            url,
            method: method.to_string(),
            payment_id,
            profile_id,
            merchant_id,
            created_at: OffsetDateTime::now_utc().unix_timestamp_nanos(),
            status_code: None,
            request_id: request_id
                .map(|i| i.to_string())
                .unwrap_or("NO_REQUEST_ID".to_string()),
            routing_engine,
            payment_connector: None,
            routing_approach: None,
        }
    }

    /// fn set_response_body
    pub fn set_response_body<T: Serialize>(&mut self, response: &T) {
        match masking::masked_serialize(response) {
            Ok(masked) => {
                self.response = Some(masked.to_string());
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
        let connectors = connectors
            .into_iter()
            .map(|c| {
                format!(
                    "{:?}:{:?}",
                    c.connector,
                    c.merchant_connector_id
                        .map(|id| id.get_string_repr().to_string())
                        .unwrap_or(String::from(""))
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        self.routable_connectors = connectors;
    }

    /// set payment connector
    pub fn set_payment_connector(&mut self, connector: RoutableConnectorChoice) {
        self.payment_connector = Some(format!(
            "{:?}:{:?}",
            connector.connector,
            connector
                .merchant_connector_id
                .map(|id| id.get_string_repr().to_string())
                .unwrap_or(String::from(""))
        ));
    }

    /// set routing approach
    pub fn set_routing_approach(&mut self, approach: String) {
        self.routing_approach = Some(approach);
    }

    /// Returns the request ID of the event.
    pub fn get_request_id(&self) -> &str {
        &self.request_id
    }

    /// Returns the merchant ID of the event.
    pub fn get_merchant_id(&self) -> &str {
        self.merchant_id.get_string_repr()
    }

    /// Returns the payment ID of the event.
    pub fn get_payment_id(&self) -> &str {
        &self.payment_id
    }

    /// Returns the profile ID of the event.
    pub fn get_profile_id(&self) -> &str {
        self.profile_id.get_string_repr()
    }
}
