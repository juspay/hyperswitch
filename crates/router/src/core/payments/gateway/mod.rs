//! Gateway Abstraction Layer
//!
//! This module provides router-specific implementations of the gateway traits
//! defined in hyperswitch_interfaces.
//!
//! The common gateway traits are now in hyperswitch_interfaces::api::gateway,
//! allowing other crates (like subscriptions) to use them without depending on router.

pub mod direct;
pub mod factory;
pub mod ucs;

use async_trait::async_trait;
use hyperswitch_interfaces::api::gateway as gateway_interface;

use crate::{
    routes::SessionState,
    types::api,
};
use hyperswitch_interfaces::configs::MerchantConnectorAccountType;

/// Re-export common gateway types from hyperswitch_interfaces
pub use gateway_interface::GatewayExecutionPath;

/// Router-specific PaymentGateway trait
///
/// This is a type alias for the generic PaymentGateway trait from hyperswitch_interfaces,
/// specialized with router-specific types.
///
/// # Type Parameters
/// * `F` - Flow type (e.g., api::Authorize, api::PSync)
/// * `Req` - Request data type (e.g., PaymentsAuthorizeData)
/// * `Resp` - Response data type (e.g., PaymentsResponseData)
#[async_trait]
pub trait PaymentGateway<F, Req, Resp>:
    gateway_interface::PaymentGateway<
        SessionState,
        api::ConnectorData,
        MerchantConnectorAccountType,
        F,
        Req,
        Resp,
    >
{
}

/// Blanket implementation for any type that implements the interface trait
impl<T, F, Req, Resp> PaymentGateway<F, Req, Resp> for T where
    T: gateway_interface::PaymentGateway<
        SessionState,
        api::ConnectorData,
        MerchantConnectorAccountType,
        F,
        Req,
        Resp,
    >
{
}

pub use direct::DirectGateway;
pub use factory::GatewayFactory;
pub use ucs::UnifiedConnectorServiceGateway;