#[cfg(feature = "v1")]
use hyperswitch_domain_models::{
    router_flow_types::subscriptions::SubscriptionCreate as SubscriptionCreateFlow,
    router_request_types::subscriptions::SubscriptionCreateRequest,
    router_response_types::subscriptions::SubscriptionCreateResponse,
};

#[cfg(feature = "v1")]
use super::{ConnectorCommon, ConnectorIntegration};

#[cfg(feature = "v1")]
/// trait SubscriptionCreate
pub trait SubscriptionCreate:
    ConnectorIntegration<SubscriptionCreateFlow, SubscriptionCreateRequest, SubscriptionCreateResponse>
{
}

/// trait Subscriptions
#[cfg(feature = "v1")]
pub trait Subscriptions: ConnectorCommon + SubscriptionCreate {}

/// trait Subscriptions
#[cfg(not(feature = "v1"))]
pub trait Subscriptions {}

/// trait SubscriptionCreate
#[cfg(not(feature = "v1"))]
pub trait SubscriptionCreate {}
