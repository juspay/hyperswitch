//! Direct Gateway Implementation
//!
//! This gateway executes payment operations through direct connector integration
//! using the traditional execute_connector_processing_step flow.

use async_trait::async_trait;
use common_enums::CallConnectorAction;
use hyperswitch_domain_models::router_data::RouterData;
use hyperswitch_interfaces::{
    api::gateway as gateway_interface, connector_integration_interface::BoxedConnectorIntegrationInterface,
};

use super::PaymentGateway;
use crate::{
    core::errors::RouterResult,
    routes::SessionState,
    services,
    types::{api, MerchantConnectorAccountType},
};

/// Gateway that executes through direct connector integration
///
/// This gateway wraps the traditional execute_connector_processing_step
/// and provides the PaymentGateway trait interface.
pub struct DirectGateway<F, ResourceCommonData, Req, Resp> {
    /// Boxed connector integration interface
    pub connector_integration:
        BoxedConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp>,
}

impl<F, ResourceCommonData, Req, Resp> DirectGateway<F, ResourceCommonData, Req, Resp> {
    /// Create a new DirectGateway with the given connector integration
    pub fn new(
        connector_integration: BoxedConnectorIntegrationInterface<
            F,
            ResourceCommonData,
            Req,
            Resp,
        >,
    ) -> Self {
        Self {
            connector_integration,
        }
    }
}

#[async_trait]
impl<F, ResourceCommonData, Req, Resp>
    gateway_interface::PaymentGateway<
        SessionState,
        api::ConnectorData,
        MerchantConnectorAccountType,
        F,
        Req,
        Resp,
    > for DirectGateway<F, ResourceCommonData, Req, Resp>
where
    F: Clone + Send + Sync + 'static,
    ResourceCommonData: Clone + Send + Sync + 'static,
    Req: Clone + Send + Sync + 'static,
    Resp: Clone + Send + Sync + 'static,
{
    async fn execute(
        &self,
        state: &SessionState,
        router_data: RouterData<F, Req, Resp>,
        _connector: &api::ConnectorData,
        _merchant_connector_account: &MerchantConnectorAccountType,
        call_connector_action: CallConnectorAction,
    ) -> Result<RouterData<F, Req, Resp>, hyperswitch_interfaces::errors::ConnectorError> {
        // Delegate to the traditional execute_connector_processing_step
        services::execute_connector_processing_step(
            state,
            self.connector_integration.clone(),
            &router_data,
            call_connector_action,
            None, // connector_request - let the integration build it
            None, // return_raw_connector_response - use default behavior
        )
        .await
        .map_err(|e| {
            // Convert RouterError to ConnectorError
            hyperswitch_interfaces::errors::ConnectorError::ProcessingStepFailed(Some(
                e.to_string(),
            ))
        })
    }
}