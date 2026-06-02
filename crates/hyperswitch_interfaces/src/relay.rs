//! Connector relay integration interface

use api_models::unreferenced_refund::UnreferencedRefundRequest;
use bytes::Bytes;
use common_utils::request::Request;
use hyperswitch_domain_models::router_data::{AccessToken, ConnectorAuthType};
use hyperswitch_masking::Secret;

use crate::errors::ConnectorError;

/// All data required to build a relay request
#[derive(Debug)]
pub struct UnreferencedRefundRouterData<'a> {
    /// Merchant-supplied request fields (amount, currency, card data, etc.)
    pub request: &'a UnreferencedRefundRequest,
    /// Access token fetched from the Redis cache by Hyperswitch — never supplied by the merchant.
    pub access_token: Option<AccessToken>,
    /// Parsed connector credentials.
    pub auth_type: &'a ConnectorAuthType,
    /// Connector base URL from config.
    pub base_url: &'a str,
}

/// Response returned by a connector for an unreferenced refund relay operation.
#[derive(Debug)]
pub struct UnreferencedRefundResponse {
    /// Connector-assigned refund/transaction ID
    pub connector_refund_id: Option<String>,
    /// Mapped refund status
    pub refund_status: common_enums::RefundStatus,
    /// Connector error code (populated on failure)
    pub error_code: Option<String>,
    /// Connector error message (populated on failure)
    pub error_message: Option<String>,
    /// Raw JSON response from the connector
    pub raw_response: Option<Secret<serde_json::Value>>,
}

/// Trait implemented by connectors that support relay operations (e.g. unreferenced refund).
pub trait ConnectorRelayIntegration {
    /// Connector base URL from config.
    fn base_url<'a>(
        &self,
        connectors: &'a hyperswitch_domain_models::connector_endpoints::Connectors,
    ) -> &'a str;

    /// Whether this connector requires an access token before relay requests.
    fn supports_access_token(&self) -> bool;

    /// Build the outbound HTTP request.
    fn build_relay_request(
        &self,
        router_data: &UnreferencedRefundRouterData<'_>,
    ) -> error_stack::Result<Request, ConnectorError>;

    /// Parse a 2xx success response — connector-specific.
    fn handle_relay_success_response(
        &self,
        response: Bytes,
    ) -> error_stack::Result<UnreferencedRefundResponse, ConnectorError>;

    /// Parse a 4xx error response — connector-specific.
    fn get_relay_error_response(
        &self,
        response: Bytes,
        status_code: u16,
    ) -> error_stack::Result<UnreferencedRefundResponse, ConnectorError>;

    /// Parse a 5xx error response — default impl returns a generic message without JSON parsing.
    fn get_relay_5xx_error_response(
        &self,
        status_code: u16,
    ) -> error_stack::Result<UnreferencedRefundResponse, ConnectorError> {
        Ok(UnreferencedRefundResponse {
            connector_refund_id: None,
            refund_status: common_enums::RefundStatus::Pending,
            error_code: Some(status_code.to_string()),
            error_message: Some(format!("Server error: HTTP {status_code}")),
            raw_response: None,
        })
    }

    /// Orchestrates the three methods above based on HTTP status code.
    /// Connectors do not override this.
    fn handle_relay_response(
        &self,
        response: Bytes,
        status_code: u16,
    ) -> error_stack::Result<UnreferencedRefundResponse, ConnectorError> {
        match status_code {
            500..=599 => self.get_relay_5xx_error_response(status_code),
            400..=499 => self.get_relay_error_response(response, status_code),
            _ => self.handle_relay_success_response(response),
        }
    }
}
