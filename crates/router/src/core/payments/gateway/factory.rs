//! Gateway Factory
//!
//! This module provides the router-specific factory implementation for creating
//! the appropriate gateway based on the execution path decision (Direct vs UCS).
//!
//! This implements the GatewayFactory trait from hyperswitch_interfaces.

use hyperswitch_domain_models::router_data::RouterData;
use hyperswitch_interfaces::api::gateway as gateway_interface;

use super::{DirectGateway, GatewayExecutionPath, PaymentGateway, UnifiedConnectorServiceGateway};
use crate::{
    core::{
        errors::RouterResult,
        payments::PaymentData,
        unified_connector_service::{self as ucs, types::ExecutionPath},
    },
    routes::SessionState,
    types::{api, MerchantConnectorAccountType},
};

/// Factory for creating payment gateways
pub struct GatewayFactory;

impl GatewayFactory {
    /// Create a gateway for Authorize flow
    ///
    /// # Arguments
    /// * `state` - Application state
    /// * `connector` - Connector data
    /// * `router_data` - Payment router data
    /// * `payment_data` - Optional payment data for decision making
    ///
    /// # Returns
    /// A boxed PaymentGateway implementation (Direct or UCS)
    pub async fn create_authorize_gateway(
        state: &SessionState,
        connector: &api::ConnectorData,
        router_data: &RouterData<
            api::Authorize,
            hyperswitch_domain_models::types::PaymentsAuthorizeData,
            hyperswitch_domain_models::types::PaymentsResponseData,
        >,
        payment_data: Option<&PaymentData>,
    ) -> RouterResult<
        Box<
            dyn PaymentGateway<
                api::Authorize,
                hyperswitch_domain_models::types::PaymentsAuthorizeData,
                hyperswitch_domain_models::types::PaymentsResponseData,
            >,
        >,
    > {
        let execution_path = Self::determine_execution_path(
            state,
            &connector.merchant_connector_id,
            router_data,
            payment_data,
        )
        .await?;

        match execution_path {
            GatewayExecutionPath::Direct => {
                // Get connector integration
                let connector_integration = connector.connector.get_connector_integration();

                Ok(Box::new(DirectGateway::new(connector_integration)))
            }
            GatewayExecutionPath::UnifiedConnectorService => {
                Ok(Box::new(UnifiedConnectorServiceGateway::new()))
            }
            GatewayExecutionPath::ShadowUnifiedConnectorService => {
                // For now, return Direct gateway
                // TODO: Implement ShadowGateway that executes both paths
                let connector_integration = connector.connector.get_connector_integration();
                Ok(Box::new(DirectGateway::new(connector_integration)))
            }
        }
    }

    /// Create a gateway for PSync flow
    pub async fn create_psync_gateway(
        state: &SessionState,
        connector: &api::ConnectorData,
        router_data: &RouterData<
            api::PSync,
            hyperswitch_domain_models::types::PaymentsSyncData,
            hyperswitch_domain_models::types::PaymentsResponseData,
        >,
        payment_data: Option<&PaymentData>,
    ) -> RouterResult<
        Box<
            dyn PaymentGateway<
                api::PSync,
                hyperswitch_domain_models::types::PaymentsSyncData,
                hyperswitch_domain_models::types::PaymentsResponseData,
            >,
        >,
    > {
        let execution_path = Self::determine_execution_path(
            state,
            &connector.merchant_connector_id,
            router_data,
            payment_data,
        )
        .await?;

        match execution_path {
            GatewayExecutionPath::Direct => {
                let connector_integration = connector.connector.get_connector_integration();
                Ok(Box::new(DirectGateway::new(connector_integration)))
            }
            GatewayExecutionPath::UnifiedConnectorService => {
                Ok(Box::new(UnifiedConnectorServiceGateway::new()))
            }
            GatewayExecutionPath::ShadowUnifiedConnectorService => {
                let connector_integration = connector.connector.get_connector_integration();
                Ok(Box::new(DirectGateway::new(connector_integration)))
            }
        }
    }

    /// Create a gateway for SetupMandate flow
    pub async fn create_setup_mandate_gateway(
        state: &SessionState,
        connector: &api::ConnectorData,
        router_data: &RouterData<
            api::SetupMandate,
            hyperswitch_domain_models::types::SetupMandateRequestData,
            hyperswitch_domain_models::types::PaymentsResponseData,
        >,
        payment_data: Option<&PaymentData>,
    ) -> RouterResult<
        Box<
            dyn PaymentGateway<
                api::SetupMandate,
                hyperswitch_domain_models::types::SetupMandateRequestData,
                hyperswitch_domain_models::types::PaymentsResponseData,
            >,
        >,
    > {
        let execution_path = Self::determine_execution_path(
            state,
            &connector.merchant_connector_id,
            router_data,
            payment_data,
        )
        .await?;

        match execution_path {
            GatewayExecutionPath::Direct => {
                let connector_integration = connector.connector.get_connector_integration();
                Ok(Box::new(DirectGateway::new(connector_integration)))
            }
            GatewayExecutionPath::UnifiedConnectorService => {
                Ok(Box::new(UnifiedConnectorServiceGateway::new()))
            }
            GatewayExecutionPath::ShadowUnifiedConnectorService => {
                let connector_integration = connector.connector.get_connector_integration();
                Ok(Box::new(DirectGateway::new(connector_integration)))
            }
        }
    }

    /// Determine the execution path (Direct vs UCS)
    ///
    /// This reuses the existing decision logic from should_call_unified_connector_service
    async fn determine_execution_path<F, Req, Resp>(
        state: &SessionState,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        router_data: &RouterData<F, Req, Resp>,
        payment_data: Option<&PaymentData>,
    ) -> RouterResult<GatewayExecutionPath> {
        // Call the existing UCS decision function
        let execution_path = ucs::should_call_unified_connector_service(
            state,
            merchant_connector_id,
            router_data,
            payment_data,
        )
        .await?;

        // Map ExecutionPath to GatewayExecutionPath
        Ok(match execution_path {
            ExecutionPath::Direct => GatewayExecutionPath::Direct,
            ExecutionPath::UnifiedConnectorService => {
                GatewayExecutionPath::UnifiedConnectorService
            }
            ExecutionPath::ShadowUnifiedConnectorService => {
                GatewayExecutionPath::ShadowUnifiedConnectorService
            }
        })
    }
}