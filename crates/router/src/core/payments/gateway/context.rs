//! Gateway execution context for router crate
//!
//! This module defines the RouterGatewayContext type, which contains all the
//! information needed for executing payment operations through either direct
//! connector integration or Unified Connector Service (UCS).

use common_enums::{ExecutionMode, ExecutionPath, GatewaySystem};
use common_utils::id_type;
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::{business_profile, payments::HeaderPayload, platform::Platform};
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
    pub fn new(
        platform: Platform,
        header_payload: HeaderPayload,
        business_profile: &business_profile::Profile,
        #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
        execution_path: ExecutionPath,
        creds_identifier: Option<String>,
    ) -> Self {
        let lineage_ids = LineageIds::new(
            business_profile.merchant_id.clone(),
            business_profile.get_id().clone(),
        );
        let execution_mode = match execution_path {
            ExecutionPath::UnifiedConnectorService => ExecutionMode::Primary,
            ExecutionPath::ShadowUnifiedConnectorService => ExecutionMode::Shadow,
            // ExecutionMode is irrelevant for Direct path in this context
            ExecutionPath::Direct => ExecutionMode::NotApplicable,
        };
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
    pub fn direct(
        platform: Platform,
        #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
        merchant_id: id_type::MerchantId,
        profile_id: id_type::ProfileId,
        creds_identifier: Option<String>,
    ) -> Self {
        let lineage_ids = LineageIds::new(merchant_id, profile_id);
        Self {
            platform,
            header_payload: HeaderPayload::default(),
            lineage_ids,
            merchant_connector_account,
            execution_mode: ExecutionMode::NotApplicable,
            execution_path: ExecutionPath::Direct,
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
