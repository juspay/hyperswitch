//! Gateway execution context for router crate
//!
//! This module defines the RouterGatewayContext type, which contains all the
//! information needed for executing payment operations through either direct
//! connector integration or Unified Connector Service (UCS).

use common_enums::{ExecutionMode, ExecutionPath};
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::{merchant_context::MerchantContext, payments::HeaderPayload};
use hyperswitch_interfaces::api::gateway::GatewayContext;

use crate::core::payments::helpers;

/// Router's gateway execution context
///
/// This is the router crate's implementation of gateway context. It contains
/// all the information needed for both direct connector execution and UCS execution.
///
/// # Type Parameters
/// * `PaymentData` - The payment data type (e.g., PaymentData<api::Authorize>)
#[derive(Clone, Debug)]
pub struct RouterGatewayContext<'a, PaymentData> {
    /// Reference to payment data
    pub payment_data: &'a PaymentData,
    
    /// Merchant context (merchant_id, profile_id, etc.)
    pub merchant_context: &'a MerchantContext,
    
    /// Header payload (x-reference-id, etc.)
    pub header_payload: &'a HeaderPayload,
    
    /// Lineage IDs for distributed tracing
    pub lineage_ids: LineageIds,
    
    /// Merchant connector account details
    #[cfg(feature = "v1")]
    pub merchant_connector_account: &'a helpers::MerchantConnectorAccountType,
    
    /// Merchant connector account details (v2)
    #[cfg(feature = "v2")]
    pub merchant_connector_account: &'a hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
    
    /// Execution mode (Primary, Shadow, etc.)
    pub execution_mode: ExecutionMode,
    
    /// Execution path (Direct, UCS, or Shadow)
    pub execution_path: ExecutionPath,
}

impl<'a, PaymentData> RouterGatewayContext<'a, PaymentData> {
    /// Create a new router gateway context
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        payment_data: &'a PaymentData,
        merchant_context: &'a MerchantContext,
        header_payload: &'a HeaderPayload,
        lineage_ids: LineageIds,
        #[cfg(feature = "v1")]
        merchant_connector_account: &'a helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: &'a hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
        execution_mode: ExecutionMode,
        execution_path: ExecutionPath,
    ) -> Self {
        Self {
            payment_data,
            merchant_context,
            header_payload,
            lineage_ids,
            merchant_connector_account,
            execution_mode,
            execution_path,
        }
    }
}

/// Implementation of GatewayContext trait for RouterGatewayContext
///
/// This allows the framework to extract execution metadata without knowing
/// the concrete structure of RouterGatewayContext.
impl<PaymentData> GatewayContext for RouterGatewayContext<'_, PaymentData>
where
    PaymentData: Clone + Send + Sync,
{
    fn execution_path(&self) -> ExecutionPath {
        self.execution_path
    }
    
    fn execution_mode(&self) -> ExecutionMode {
        self.execution_mode
    }
}