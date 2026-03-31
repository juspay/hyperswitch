//! Gateway context for payout operations
//!
//! This module provides the context structure needed for payout gateway execution,
//! similar to the payment gateway context.

use common_enums::{ExecutionMode, ExecutionPath};
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::payments::HeaderPayload;

use crate::core::payments::helpers::MerchantConnectorAccountType;

/// Context information required for payout gateway execution
#[derive(Clone)]
pub struct RouterGatewayContext {
    /// Credentials identifier for connector authentication
    pub creds_identifier: Option<String>,
    /// Processor information
    pub processor: hyperswitch_domain_models::platform::Processor,
    /// HTTP header payload from the request
    pub header_payload: HeaderPayload,
    /// Lineage IDs for tracing and tracking
    pub lineage_ids: LineageIds,
    /// Merchant connector account details
    pub merchant_connector_account: MerchantConnectorAccountType,
    /// Execution path (Direct, UCS, or Shadow UCS)
    pub execution_path: ExecutionPath,
    /// Execution mode (Primary or Shadow)
    pub execution_mode: ExecutionMode,
}
impl RouterGatewayContext {
    /// Get the gateway system (Direct, UnifiedConnectorService, etc.)
    pub fn get_gateway_system(&self) -> common_enums::GatewaySystem {
        match self.execution_path {
            ExecutionPath::Direct => common_enums::GatewaySystem::Direct,
            ExecutionPath::UnifiedConnectorService => {
                common_enums::GatewaySystem::UnifiedConnectorService
            }
            ExecutionPath::ShadowUnifiedConnectorService => common_enums::GatewaySystem::Direct,
        }
    }
}
