//! Subscriptions Interface for V1

use hyperswitch_domain_models::{
    router_flow_types::subscriptions::GetSubscriptionPlans,
    router_request_types::subscriptions::GetSubscriptionPlansRequest,
    router_response_types::subscriptions::GetSubscriptionPlansResponse,
};

use super::{ConnectorCommon, ConnectorIntegration};

#[cfg(feature = "v1")]
/// trait GetSubscriptionPlans for V1
pub trait GetSubscriptionPlansFlow:
    ConnectorIntegration<
    GetSubscriptionPlans,
    GetSubscriptionPlansRequest,
    GetSubscriptionPlansResponse,
>
{
}

/// trait Subscriptions
#[cfg(feature = "v1")]
pub trait Subscriptions: ConnectorCommon + GetSubscriptionPlansFlow {}

/// trait Subscriptions (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait Subscriptions {}

/// trait GetSubscriptionPlansFlow (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait GetSubscriptionPlansFlow {}
