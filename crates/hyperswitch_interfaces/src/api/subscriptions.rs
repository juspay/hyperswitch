//! Subscriptions Interface for V1
#[cfg(feature = "v1")]
use hyperswitch_domain_models::{
    router_flow_types::subscriptions::GetSubscriptionPlanPrices,
    router_request_types::subscriptions::GetSubscriptionPlanPricesRequest,
    router_response_types::subscriptions::GetSubscriptionPlanPricesResponse,
};

#[cfg(feature = "v1")]
use super::{ConnectorCommon, ConnectorIntegration};

#[cfg(feature = "v1")]
/// trait GetSubscriptionPlanPrices for V1
pub trait GetSubscriptionPlanPricesFlow:
    ConnectorIntegration<
    GetSubscriptionPlanPrices,
    GetSubscriptionPlanPricesRequest,
    GetSubscriptionPlanPricesResponse,
>
{
}

/// trait Subscriptions
#[cfg(feature = "v1")]
pub trait Subscriptions: ConnectorCommon + GetSubscriptionPlanPricesFlow {}

/// trait Subscriptions (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait Subscriptions {}

/// trait GetSubscriptionPlanPricesFlow (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait GetSubscriptionPlanPricesFlow {}
