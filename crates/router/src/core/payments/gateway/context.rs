//! Gateway execution context for router crate
//!
//! This module defines the RouterGatewayContext type, which contains all the
//! information needed for executing payment operations through either direct
//! connector integration or Unified Connector Service (UCS).

use common_enums::{ExecutionMode, ExecutionPath, GatewaySystem};
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::{payments::HeaderPayload, platform::Platform};
use hyperswitch_interfaces::api::gateway::GatewayContext;

use crate::core::payments::helpers;

/// Router's gateway execution context
///
/// This is the router crate's implementation of gateway context. It contains
/// all the information needed for both direct connector execution and UCS execution.
#[derive(Clone, Debug)]
pub struct RouterGatewayContext {
    pub creds_identifier: Option<String>,
    /// Merchant context (merchant_id, profile_id, etc.)
    pub platform: Platform,

    /// Header payload (x-reference-id, etc.)
    pub header_payload: HeaderPayload,

    /// Lineage IDs for distributed tracing
    pub lineage_ids: LineageIds,

    /// Merchant connector account details
    #[cfg(feature = "v1")]
    pub merchant_connector_account: helpers::MerchantConnectorAccountType,

    /// Merchant connector account details (v2)
    #[cfg(feature = "v2")]
    pub merchant_connector_account:
        hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,

    /// Execution mode (Primary, Shadow, etc.)
    pub execution_mode: ExecutionMode,

    /// Execution path (Direct, UCS, or Shadow)
    pub execution_path: ExecutionPath,
}

impl RouterGatewayContext {
    /// Create a new router gateway context
    pub fn new(
        platform: Platform,
        header_payload: HeaderPayload,
        lineage_ids: LineageIds,
        #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
        execution_mode: ExecutionMode,
        execution_path: ExecutionPath,
        creds_identifier: Option<String>,
    ) -> Self {
        Self {
            platform,
            header_payload,
            lineage_ids,
            merchant_connector_account,
            execution_mode,
            execution_path,
            creds_identifier,
        }
    }
}

/// Implementation of GatewayContext trait for RouterGatewayContext
///
/// This allows the framework to extract execution metadata without knowing
/// the concrete structure of RouterGatewayContext.
impl GatewayContext for RouterGatewayContext {
    fn execution_path(&self) -> ExecutionPath {
        self.execution_path
    }

    /// Get the execution mode (Primary, Shadow, etc.)
    fn execution_mode(&self) -> ExecutionMode {
        self.execution_mode
    }
}
impl RouterGatewayContext {
    /// Get the gateway system (Direct, UnifiedConnectorService, etc.)
    pub fn get_gateway_system(&self) -> GatewaySystem {
        match self.execution_path {
            ExecutionPath::Direct => GatewaySystem::Direct,
            ExecutionPath::UnifiedConnectorService => GatewaySystem::UnifiedConnectorService,
            ExecutionPath::ShadowUnifiedConnectorService => GatewaySystem::Direct,
        }
    }
}
