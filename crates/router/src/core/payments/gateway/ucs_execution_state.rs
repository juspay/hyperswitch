//! UCS Execution State
//!
//! This module provides a wrapper around SessionState that includes all
//! context needed for UCS flow execution.

use common_enums::ExecutionMode;
use common_utils::errors::CustomResult;
use external_services::grpc_client::{LineageIds, UnifiedConnectorServiceClient};
use hyperswitch_domain_models::{merchant_context::MerchantContext, payments::HeaderPayload};
use hyperswitch_interfaces::{errors::ConnectorError, unified_connector_service::UcsStateProvider};

use crate::{core::payments::helpers, routes::SessionState};

use super::ucs_context::RouterUcsContext;

/// Execution state for UCS flows containing all required context
pub struct UcsExecutionState<'a> {
    pub session_state: &'a SessionState,
    pub merchant_context: &'a MerchantContext,
    pub header_payload: &'a HeaderPayload,
    pub lineage_ids: LineageIds,
    #[cfg(feature = "v1")]
    pub merchant_connector_account: &'a helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")]
    pub merchant_connector_account:
        &'a hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
    pub execution_mode: ExecutionMode,
}

impl<'a> UcsExecutionState<'a> {
    /// Create a new UCS execution state
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_state: &'a SessionState,
        merchant_context: &'a MerchantContext,
        header_payload: &'a HeaderPayload,
        lineage_ids: LineageIds,
        #[cfg(feature = "v1")] merchant_connector_account: &'a helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: &'a hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
        execution_mode: ExecutionMode,
    ) -> Self {
        Self {
            session_state,
            merchant_context,
            header_payload,
            lineage_ids,
            merchant_connector_account,
            execution_mode,
        }
    }
}

impl<'a> UcsStateProvider for UcsExecutionState<'a> {
    type GrpcClient = UnifiedConnectorServiceClient;
    type Context = RouterUcsContext;
    type MerchantContext = MerchantContext;
    type HeaderPayload = HeaderPayload;
    type LineageIds = LineageIds;
    #[cfg(feature = "v1")]
    type MerchantConnectorAccount = helpers::MerchantConnectorAccountType;
    #[cfg(feature = "v2")]
    type MerchantConnectorAccount =
        hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails;
    type EventHandler = dyn common_utils::events::EventHandler;

    fn get_ucs_client(&self) -> CustomResult<&Self::GrpcClient, ConnectorError> {
        self.session_state
            .grpc_client
            .unified_connector_service_client
            .as_ref()
            .ok_or(ConnectorError::ProcessingStepFailed(Some(
                "Failed to fetch Unified Connector Service client"
                    .to_string()
                    .into(),
            )))
            .map_err(Into::into)
    }

    fn get_merchant_context(&self) -> &Self::MerchantContext {
        self.merchant_context
    }

    fn get_header_payload(&self) -> &Self::HeaderPayload {
        self.header_payload
    }

    fn get_lineage_ids(&self) -> Self::LineageIds {
        self.lineage_ids.clone()
    }

    fn get_merchant_connector_account(&self) -> &Self::MerchantConnectorAccount {
        self.merchant_connector_account
    }

    fn get_execution_mode(&self) -> ExecutionMode {
        self.execution_mode
    }

    fn build_ucs_context(&self) -> CustomResult<Self::Context, ConnectorError> {
        let (_, auth, headers_builder) = super::helpers::prepare_ucs_infrastructure(
            self.session_state,
            self.merchant_context,
            self.header_payload,
            self.lineage_ids.clone(),
            self.merchant_connector_account,
            self.execution_mode,
        )?;

        let headers = headers_builder.build();
        Ok(RouterUcsContext::new(auth, headers, self.lineage_ids.clone()))
    }

    fn get_event_handler(&self) -> &Self::EventHandler {
        &self.session_state.event_handler
    }
}