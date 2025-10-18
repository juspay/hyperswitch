//! Direct Gateway Implementation
//!
//! NOTE: This gateway has a fundamental design limitation due to ownership constraints.
//! The `execute_connector_processing_step` function takes ownership of the
//! `BoxedConnectorIntegrationInterface`, which cannot be cloned (it's a Box<dyn Trait>).
//!
//! This means the gateway cannot be reused across multiple calls, which defeats
//! the purpose of the gateway abstraction.
//!
//! RECOMMENDATION: Do not use this gateway pattern. Instead, call
//! `execute_connector_processing_step` directly in the payment flow where you
//! can obtain a fresh connector integration for each call.

use async_trait::async_trait;
use common_enums::CallConnectorAction;
use hyperswitch_domain_models::router_data::RouterData;
use hyperswitch_interfaces::{
    api::gateway as gateway_interface,
    connector_integration_interface,
};
use common_utils::errors::CustomResult;
use crate::{routes::SessionState, services, types::api};
use hyperswitch_interfaces::configs::MerchantConnectorAccountType;

/// Gateway that executes through direct connector integration
///
/// WARNING: This gateway has ownership constraints that prevent it from being
/// reused. It can only execute once before the connector_integration is consumed.
pub struct DirectGateway<F, ResourceCommonData, Req, Resp> {
    /// Boxed connector integration interface (consumed on first execute)
    connector_integration:
        connector_integration_interface::BoxedConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp>,
}

impl<F, ResourceCommonData, Req, Resp> DirectGateway<F, ResourceCommonData, Req, Resp> {
    /// Create a new DirectGateway with the given connector integration
    ///
    /// Note: This gateway can only be used once due to ownership constraints
    pub fn new(
        connector_integration: connector_integration_interface::BoxedConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp>,
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
    F: Clone + Send + Sync + std::fmt::Debug + 'static,
    ResourceCommonData: Clone + Send + Sync + 'static + connector_integration_interface::RouterDataConversion<F, Req, Resp>,
    Req: Clone + Send + Sync + std::fmt::Debug + 'static,
    Resp: Clone + Send + Sync + std::fmt::Debug + 'static,
{
    async fn execute(
        self,
        state: &SessionState,
        router_data: RouterData<F, Req, Resp>,
        _connector: &api::ConnectorData,
        _merchant_connector_account: &MerchantConnectorAccountType,
        _call_connector_action: CallConnectorAction,
    ) -> CustomResult<RouterData<F, Req, Resp>, hyperswitch_interfaces::errors::ConnectorError> {
        services::execute_connector_processing_step(
            state,
            self.connector_integration,
            &router_data,
            CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        
    }
}