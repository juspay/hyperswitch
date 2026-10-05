//! Gateway execution context owned by payout operations.
//!
//! Payout routing and vault configuration stay separate from payment gateway state.

use common_enums::{ExecutionMode, ExecutionPath, GatewaySystem};
use common_utils::errors::CustomResult;
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::{payments::HeaderPayload, platform::Processor};
use hyperswitch_interfaces::{api::gateway::GatewayContext, errors::ConnectorError};

use crate::{core::unified_connector_service::kill_switch::RolloutSettings, routes::SessionState};

/// Request-scoped routing and authentication information for payout gateways.
#[derive(Clone, Debug)]
pub struct RouterGatewayContext {
    pub creds_identifier: Option<String>,
    pub processor: Processor,
    pub header_payload: HeaderPayload,
    pub lineage_ids: LineageIds,
    #[cfg(feature = "v1")]
    pub merchant_connector_account: crate::core::payments::helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")]
    pub merchant_connector_account:
        hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
    pub execution_path: ExecutionPath,
    pub execution_mode: ExecutionMode,
    /// Preserve the rollout decision for normal payout calls and logging.
    pub kill_switch_enabled: bool,
    pub kill_switch_threshold: u64,
    pub connector_decline_threshold: Option<u64>,
    pub rollout_scope: Option<String>,
    /// Vault configuration only; opaque tokens are carried separately in PayoutData.
    #[cfg(feature = "v1")]
    pub payout_execution_context: crate::core::payouts::proxy::PayoutExecutionContext,
}

impl GatewayContext for RouterGatewayContext {
    fn execution_path(&self) -> ExecutionPath {
        // Proxy payouts must never select Direct, including the shadow primary path.
        // The UCS gateway rejects inconsistent stored routing fields before encoding.
        #[cfg(feature = "v1")]
        if !matches!(
            self.payout_execution_context,
            crate::core::payouts::proxy::PayoutExecutionContext::Normal
        ) {
            return ExecutionPath::UnifiedConnectorService;
        }
        self.execution_path
    }

    fn execution_mode(&self) -> ExecutionMode {
        self.execution_mode
    }
}

impl RouterGatewayContext {
    pub fn rollout_settings(&self) -> RolloutSettings {
        RolloutSettings {
            execution_mode: self.execution_mode,
            kill_switch_enabled: self.kill_switch_enabled,
            kill_switch_threshold: self.kill_switch_threshold,
            connector_decline_threshold: self.connector_decline_threshold,
            rollout_scope: self.rollout_scope.clone(),
        }
    }

    pub fn get_gateway_system(&self) -> GatewaySystem {
        match self.execution_path() {
            ExecutionPath::Direct | ExecutionPath::ShadowUnifiedConnectorService => {
                GatewaySystem::Direct
            }
            ExecutionPath::UnifiedConnectorService => GatewaySystem::UnifiedConnectorService,
        }
    }

    /// Shared metadata plumbing for payout gateways. Fail closed until the token-aware
    /// payout protobuf/mappings exist; never submit a normal CardPayout with vault config.
    pub fn payout_external_vault_proxy_metadata(
        &self,
        _state: &SessionState,
    ) -> CustomResult<Option<String>, ConnectorError> {
        #[cfg(feature = "v1")]
        {
            if !matches!(
                self.payout_execution_context,
                crate::core::payouts::proxy::PayoutExecutionContext::Normal
            ) {
                if self.execution_path != ExecutionPath::UnifiedConnectorService
                    || self.execution_mode != ExecutionMode::Primary
                {
                    return Err(ConnectorError::RequestEncodingFailed.into());
                }
                crate::core::payouts::proxy::ensure_proxy_transport_available(_state)
                    .map_err(|err| err.change_context(ConnectorError::RequestEncodingFailed))?;
            }
            self.payout_execution_context
                .external_vault_proxy_metadata(_state)
                .map_err(|err| err.change_context(ConnectorError::RequestEncodingFailed))
        }
        #[cfg(feature = "v2")]
        {
            Ok(None)
        }
    }
}
