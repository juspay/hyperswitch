//! Payout-owned routing and vault context.

use common_enums::{ExecutionMode, ExecutionPath, GatewaySystem};
use common_utils::errors::CustomResult;
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::{payments::HeaderPayload, platform::Processor};
use hyperswitch_interfaces::{api::gateway::GatewayContext, errors::ConnectorError};

use crate::{
    core::{self, unified_connector_service::kill_switch::RolloutSettings},
    routes::SessionState,
};

#[derive(Clone, Debug)]
pub struct RouterGatewayContext {
    pub creds_identifier: Option<String>,
    pub processor: Processor,
    pub header_payload: HeaderPayload,
    pub lineage_ids: LineageIds,
    #[cfg(feature = "v1")]
    pub merchant_connector_account: core::payments::helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")]
    pub merchant_connector_account:
        hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
    pub execution_path: ExecutionPath,
    pub execution_mode: ExecutionMode,
    /// Normal-payout rollout only; proxy payouts disable fallback.
    pub kill_switch_enabled: bool,
    pub kill_switch_threshold: u64,
    pub connector_decline_threshold: Option<u64>,
    pub rollout_scope: Option<String>,
    /// Vault configuration only; tokens remain in PayoutData.
    #[cfg(feature = "v1")]
    pub payout_execution_context: core::payouts::proxy::PayoutExecutionContext,
}

impl GatewayContext for RouterGatewayContext {
    fn execution_path(&self) -> ExecutionPath {
        // Proxy payouts cannot select Direct, even as a shadow primary.
        #[cfg(feature = "v1")]
        {
            match self.payout_execution_context {
                core::payouts::proxy::PayoutExecutionContext::Normal => self.execution_path,
                core::payouts::proxy::PayoutExecutionContext::ExternalVaultProxy { .. } => {
                    ExecutionPath::UnifiedConnectorService
                }
            }
        }
        #[cfg(feature = "v2")]
        {
            self.execution_path
        }
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

    pub fn payout_external_vault_proxy_metadata(
        &self,
        _state: &SessionState,
    ) -> CustomResult<Option<String>, ConnectorError> {
        #[cfg(feature = "v1")]
        {
            match &self.payout_execution_context {
                core::payouts::proxy::PayoutExecutionContext::Normal => Ok(None),
                context @ core::payouts::proxy::PayoutExecutionContext::ExternalVaultProxy {
                    ..
                } => match (self.execution_path, self.execution_mode) {
                    (ExecutionPath::UnifiedConnectorService, ExecutionMode::Primary) => context
                        .external_vault_proxy_metadata(_state)
                        .map_err(|err| err.change_context(ConnectorError::RequestEncodingFailed)),
                    _ => Err(ConnectorError::RequestEncodingFailed.into()),
                },
            }
        }
        #[cfg(feature = "v2")]
        {
            Ok(None)
        }
    }
}

/// Adapt shared OAuth fields without moving payout vault state into payment context.
impl From<&RouterGatewayContext> for core::payments::gateway::context::RouterGatewayContext {
    fn from(context: &RouterGatewayContext) -> Self {
        Self {
            creds_identifier: context.creds_identifier.clone(),
            processor: context.processor.clone(),
            header_payload: context.header_payload.clone(),
            lineage_ids: context.lineage_ids.clone(),
            merchant_connector_account: context.merchant_connector_account.clone(),
            execution_path: context.execution_path(),
            execution_mode: context.execution_mode,
            kill_switch_enabled: context.kill_switch_enabled,
            kill_switch_threshold: context.kill_switch_threshold,
            connector_decline_threshold: context.connector_decline_threshold,
            rollout_scope: context.rollout_scope.clone(),
        }
    }
}
