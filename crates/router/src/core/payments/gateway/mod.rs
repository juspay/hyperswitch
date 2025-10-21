//! Gateway Abstraction Layer
//!
//! This module provides router-specific implementations of the gateway traits
//! defined in hyperswitch_interfaces.
//!
//! The common gateway traits are now in hyperswitch_interfaces::api::gateway,
//! allowing other crates (like subscriptions) to use them without depending on router.

use async_trait::async_trait;
use hyperswitch_interfaces::api::gateway::{self as gateway_interface};
use hyperswitch_interfaces::connector_integration_interface::RouterDataConversion;

use crate::core::payments::PaymentData;
use crate::routes::SessionState;

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
pub trait PaymentGateway<F, RouterCommonData, Req, Resp>:
    gateway_interface::PaymentGateway<
        SessionState,
        RouterCommonData,
        F,
        Req,
        Resp,
        PaymentData<F>
    >
where
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    RouterCommonData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
{
}

/// Blanket implementation for any type that implements the interface trait
impl<T, F, RouterCommonData, Req, Resp> PaymentGateway<F, RouterCommonData, Req, Resp> for T
where
    T: gateway_interface::PaymentGateway<
        SessionState,
        RouterCommonData,
        F,
        Req,
        Resp,
        PaymentData<F>
    >,
    F: Clone + std::fmt::Debug + Send + Sync + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    RouterCommonData: Clone + RouterDataConversion<F, Req, Resp> + Send + Sync + 'static,
{
}