//! Gateway Factory
//!
//! This module provides the router-specific factory implementation for creating
//! the appropriate gateway based on the execution path decision (Direct vs UCS).
//!
//! This implements the GatewayFactory trait from hyperswitch_interfaces.

use error_stack::ResultExt;
use hyperswitch_domain_models::router_data::RouterData;

use super::{DirectGateway, GatewayExecutionPath, PaymentGateway, UnifiedConnectorServiceGateway};
use crate::{
    core::{
        errors::RouterResult,
        payments::PaymentData,
    },
    routes::SessionState,
    types::api,
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
            hyperswitch_domain_models::router_request_types::PaymentsAuthorizeData,
            hyperswitch_domain_models::router_response_types::PaymentsResponseData,
        >,
        payment_data: Option<&PaymentData<api::Authorize>>,
    ) -> RouterResult<
        Box<
            dyn PaymentGateway<
                api::Authorize,
                hyperswitch_domain_models::router_request_types::PaymentsAuthorizeData,
                hyperswitch_domain_models::router_response_types::PaymentsResponseData,
            >,
        >,
    > {
        let merchant_connector_id = connector
            .merchant_connector_id
            .as_ref()
            .ok_or(crate::core::errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Missing merchant_connector_id")?;

        let execution_path = Self::determine_execution_path(
            state,
            merchant_connector_id,
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
            hyperswitch_domain_models::router_request_types::PaymentsSyncData,
            hyperswitch_domain_models::router_response_types::PaymentsResponseData,
        >,
        payment_data: Option<&PaymentData<api::PSync>>,
    ) -> RouterResult<
        Box<
            dyn PaymentGateway<
                api::PSync,
                hyperswitch_domain_models::router_request_types::PaymentsSyncData,
                hyperswitch_domain_models::router_response_types::PaymentsResponseData,
            >,
        >,
    > {
        let merchant_connector_id = connector
            .merchant_connector_id
            .as_ref()
            .ok_or(crate::core::errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Missing merchant_connector_id")?;

        let execution_path = Self::determine_execution_path(
            state,
            merchant_connector_id,
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
            hyperswitch_domain_models::router_request_types::SetupMandateRequestData,
            hyperswitch_domain_models::router_response_types::PaymentsResponseData,
        >,
        payment_data: Option<&PaymentData<api::SetupMandate>>,
    ) -> RouterResult<
        Box<
            dyn PaymentGateway<
                api::SetupMandate,
                hyperswitch_domain_models::router_request_types::SetupMandateRequestData,
                hyperswitch_domain_models::router_response_types::PaymentsResponseData,
            >,
        >,
    > {
        let merchant_connector_id = connector
            .merchant_connector_id
            .as_ref()
            .ok_or(crate::core::errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Missing merchant_connector_id")?;

        let execution_path = Self::determine_execution_path(
            state,
            merchant_connector_id,
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
    /// NOTE: Currently always returns Direct because UCS gateway requires MerchantContext
    /// and PaymentData which are not available through the gateway trait interface.
    /// UCS decision logic should be called in the payment flow before gateway creation.
    async fn determine_execution_path<F, Req, Resp>(
        _state: &SessionState,
        _merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        _router_data: &RouterData<F, Req, Resp>,
        _payment_data: Option<&PaymentData<F>>,
    ) -> RouterResult<GatewayExecutionPath>
    where
        F: Clone,
        Req: Clone,
        Resp: Clone,
    {
        // TODO: Implement proper UCS decision logic when gateway trait is extended
        // to provide MerchantContext and full PaymentData
        Ok(GatewayExecutionPath::Direct)
    }
}