//! Gateway execution context for router crate
//!
//! This module defines the RouterGatewayContext type, which contains all the
//! information needed for executing payment operations through either direct
//! connector integration or Unified Connector Service (UCS).

use common_enums::{ExecutionMode, ExecutionPath, GatewaySystem};
use common_utils::id_type;
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::{payments::HeaderPayload, platform::Processor};
use hyperswitch_interfaces::api::gateway::GatewayContext;

use crate::core::payments::helpers;

/// Router's gateway execution context
///
/// This is the router crate's implementation of gateway context. It contains
/// all the information needed for both direct connector execution and UCS execution.
#[derive(Clone, Debug)]
pub struct RouterGatewayContext {
    pub creds_identifier: Option<String>,
    /// Processor context for payment execution
    pub processor: Processor,

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

    /// Kill switch thresholds for this scope, read once by the gate. Carried so a failure
    /// is counted against the same threshold the gate used, rather than re-reading the
    /// config later and risking a different answer.
    pub kill_switch_enabled: bool,
    pub kill_switch_threshold: u64,
    /// `None` means connector declines never trip this scope.
    pub connector_decline_threshold: Option<u64>,
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
    /// Context for the Direct path, which never calls UCS and so never trips the kill
    /// switch; the thresholds are inert here.
    pub fn direct(
        processor: Processor,
        #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
        merchant_id: id_type::MerchantId,
        profile_id: id_type::ProfileId,
        creds_identifier: Option<String>,
    ) -> Self {
        let lineage_ids = LineageIds::new(merchant_id, profile_id);
        Self {
            processor,
            header_payload: HeaderPayload::default(),
            lineage_ids,
            merchant_connector_account,
            execution_mode: ExecutionMode::NotApplicable,
            execution_path: ExecutionPath::Direct,
            creds_identifier,
            kill_switch_enabled: false,
            kill_switch_threshold: 1,
            connector_decline_threshold: None,
        }
    }

    /// Get the gateway system (Direct, UnifiedConnectorService, etc.)
    /// The kill switch settings this request resolved, for the UCS logging wrappers.
    pub fn rollout_settings(
        &self,
    ) -> crate::core::unified_connector_service::kill_switch::RolloutSettings {
        crate::core::unified_connector_service::kill_switch::RolloutSettings {
            execution_mode: self.execution_mode,
            kill_switch_enabled: self.kill_switch_enabled,
            kill_switch_threshold: self.kill_switch_threshold,
            connector_decline_threshold: self.connector_decline_threshold,
        }
    }

    pub fn get_gateway_system(&self) -> GatewaySystem {
        match self.execution_path {
            ExecutionPath::Direct => GatewaySystem::Direct,
            ExecutionPath::UnifiedConnectorService => GatewaySystem::UnifiedConnectorService,
            ExecutionPath::ShadowUnifiedConnectorService => GatewaySystem::Direct,
        }
    }
}
