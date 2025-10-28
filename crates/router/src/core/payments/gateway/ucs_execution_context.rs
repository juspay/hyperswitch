//! UCS Execution Context
//!
//! This module provides the execution context for UCS flows,
//! grouping all required parameters into a single struct.

use common_enums::ExecutionMode;
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::{merchant_context::MerchantContext, payments::HeaderPayload};
use hyperswitch_interfaces::unified_connector_service::UcsExecutionContextProvider;

use crate::core::payments::helpers;

/// Execution context for UCS flows
pub struct RouterUcsExecutionContext<'a> {
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

impl<'a> RouterUcsExecutionContext<'a> {
    /// Create a new UCS execution context
    pub fn new(
        merchant_context: &'a MerchantContext,
        header_payload: &'a HeaderPayload,
        lineage_ids: LineageIds,
        #[cfg(feature = "v1")] merchant_connector_account: &'a helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: &'a hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
        execution_mode: ExecutionMode,
    ) -> Self {
        Self {
            merchant_context,
            header_payload,
            lineage_ids,
            merchant_connector_account,
            execution_mode,
        }
    }
}

impl<'a> UcsExecutionContextProvider for RouterUcsExecutionContext<'a> {
    type MerchantContext = MerchantContext;
    type HeaderPayload = HeaderPayload;
    type LineageIds = LineageIds;
    #[cfg(feature = "v1")]
    type MerchantConnectorAccount = helpers::MerchantConnectorAccountType;
    #[cfg(feature = "v2")]
    type MerchantConnectorAccount =
        hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails;

    fn merchant_context(&self) -> &Self::MerchantContext {
        self.merchant_context
    }

    fn header_payload(&self) -> &Self::HeaderPayload {
        self.header_payload
    }

    fn lineage_ids(&self) -> Self::LineageIds {
        self.lineage_ids.clone()
    }

    fn merchant_connector_account(&self) -> &Self::MerchantConnectorAccount {
        self.merchant_connector_account
    }

    fn execution_mode(&self) -> ExecutionMode {
        self.execution_mode
    }
}