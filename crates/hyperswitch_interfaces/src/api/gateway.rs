//! Gateway Abstraction Layer
//!
//! This module provides a unified interface for executing payment operations
//! through either Direct connector integration or Unified Connector Service (UCS).
//!
//! The gateway layer abstracts the execution path decision logic, allowing payment flows
//! to use a consistent API regardless of the underlying execution path.
//!
//! This is a common abstraction that can be used by multiple services (router, subscriptions, etc.)
//! without creating circular dependencies.

use async_trait::async_trait;
use common_enums::CallConnectorAction;
use hyperswitch_domain_models::router_data::RouterData;

use crate::errors::ConnectorError;

/// Gateway execution path
///
/// Determines how the payment operation should be executed:
/// - Direct: Through direct connector integration
/// - UnifiedConnectorService: Through UCS (gRPC service)
/// - ShadowUnifiedConnectorService: Execute both paths for comparison/migration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayExecutionPath {
    /// Execute through direct connector integration
    Direct,
    /// Execute through Unified Connector Service
    UnifiedConnectorService,
    /// Execute through direct with shadow UCS execution for comparison
    ShadowUnifiedConnectorService,
}

/// Core trait for payment gateway execution
///
/// This trait provides a unified interface for executing payment operations
/// regardless of whether they go through Direct connector integration or UCS.
///
/// # Type Parameters
/// * `State` - Application state type (e.g., SessionState, SubscriptionState)
/// * `ConnectorData` - Connector information type
/// * `MerchantConnectorAccount` - Merchant connector account type
/// * `F` - Flow type (e.g., api::Authorize, api::PSync)
/// * `Req` - Request data type (e.g., PaymentsAuthorizeData)
/// * `Resp` - Response data type (e.g., PaymentsResponseData)
///
/// # Example
/// ```ignore
/// // Router implementation
/// impl PaymentGateway<SessionState, api::ConnectorData, MerchantConnectorAccountType, ...>
///     for DirectGateway { ... }
///
/// // Subscriptions implementation
/// impl PaymentGateway<SubscriptionState, SubscriptionConnectorData, SubscriptionMCAType, ...>
///     for SubscriptionGateway { ... }
/// ```
#[async_trait]
pub trait PaymentGateway<State, ConnectorData, MerchantConnectorAccount, F, Req, Resp>:
    Send + Sync
{
    /// Execute the payment operation through this gateway
    ///
    /// # Arguments
    /// * `state` - Application state containing configuration and clients
    /// * `router_data` - Payment context and request data
    /// * `connector` - Connector information
    /// * `merchant_connector_account` - Merchant's connector account details
    /// * `call_connector_action` - Action to perform (Trigger, HandleResponse, etc.)
    ///
    /// # Returns
    /// Updated RouterData with response or error
    async fn execute(
        &self,
        state: &State,
        router_data: RouterData<F, Req, Resp>,
        connector: &ConnectorData,
        merchant_connector_account: &MerchantConnectorAccount,
        call_connector_action: CallConnectorAction,
    ) -> Result<RouterData<F, Req, Resp>, ConnectorError>;
}

/// Factory trait for creating payment gateways
///
/// This trait allows different services to implement their own gateway creation logic
/// while maintaining a consistent interface.
///
/// # Type Parameters
/// * `State` - Application state type
/// * `ConnectorData` - Connector information type
/// * `PaymentData` - Payment data type used for decision making
/// * `F` - Flow type
/// * `Req` - Request data type
/// * `Resp` - Response data type
///
/// # Example
/// ```ignore
/// // Router implementation
/// impl GatewayFactory<SessionState, api::ConnectorData, PaymentData, ...>
///     for RouterGatewayFactory {
///     async fn create_gateway(...) -> Box<dyn PaymentGateway<...>> {
///         // Router-specific logic to choose Direct vs UCS
///     }
/// }
/// ```
#[async_trait]
pub trait GatewayFactory<State, ConnectorData, PaymentData, F, Req, Resp>: Send + Sync {
    /// Create a gateway for the given flow
    ///
    /// # Arguments
    /// * `state` - Application state
    /// * `connector` - Connector data
    /// * `router_data` - Payment router data
    /// * `payment_data` - Optional payment data for decision making
    ///
    /// # Returns
    /// A boxed PaymentGateway implementation (Direct, UCS, or Shadow)
    async fn create_gateway(
        &self,
        state: &State,
        connector: &ConnectorData,
        router_data: &RouterData<F, Req, Resp>,
        payment_data: Option<&PaymentData>,
    ) -> Result<Box<dyn PaymentGateway<State, ConnectorData, Self::MerchantConnectorAccount, F, Req, Resp>>, ConnectorError>;

    /// Associated type for merchant connector account
    type MerchantConnectorAccount;
}

/// Trait for determining gateway execution path
///
/// This trait encapsulates the decision logic for choosing between
/// Direct, UCS, or Shadow execution paths.
///
/// # Type Parameters
/// * `State` - Application state type
/// * `ConnectorData` - Connector information type
/// * `PaymentData` - Payment data type
/// * `F` - Flow type
/// * `Req` - Request data type
/// * `Resp` - Response data type
#[async_trait]
pub trait GatewayExecutionPathDecider<State, ConnectorData, PaymentData, F, Req, Resp>:
    Send + Sync
{
    /// Determine the execution path for the given payment operation
    ///
    /// # Arguments
    /// * `state` - Application state
    /// * `connector` - Connector data
    /// * `router_data` - Payment router data
    /// * `payment_data` - Optional payment data for decision making
    ///
    /// # Returns
    /// The execution path to use (Direct, UCS, or Shadow)
    async fn determine_execution_path(
        &self,
        state: &State,
        connector: &ConnectorData,
        router_data: &RouterData<F, Req, Resp>,
        payment_data: Option<&PaymentData>,
    ) -> Result<GatewayExecutionPath, ConnectorError>;
}