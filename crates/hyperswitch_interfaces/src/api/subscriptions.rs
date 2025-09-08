use hyperswitch_domain_models::{
    router_flow_types::subscriptions::{
        SubscriptionCreate as SubscriptionCreateFlow,
        SubscriptionRecordBack as SubscriptionRecordBackFlow,
    },
    router_request_types::subscriptions::{
        SubscriptionCreateRequest, SubscriptionsRecordBackRequest,
    },
    router_response_types::{
        revenue_recovery::RevenueRecoveryRecordBackResponse,
        subscriptions::SubscriptionCreateResponse,
    },
};

use super::{ConnectorCommon, ConnectorIntegration};

#[cfg(feature = "v1")]
/// trait SubscriptionRecordBack
pub trait SubscriptionRecordBack:
    ConnectorIntegration<
    SubscriptionRecordBackFlow,
    SubscriptionsRecordBackRequest,
    RevenueRecoveryRecordBackResponse,
>
{
}

#[cfg(feature = "v1")]
/// trait SubscriptionCreate
pub trait SubscriptionCreate:
    ConnectorIntegration<SubscriptionCreateFlow, SubscriptionCreateRequest, SubscriptionCreateResponse>
{
}

/// trait Subscriptions
#[cfg(feature = "v1")]
pub trait Subscriptions: ConnectorCommon + SubscriptionRecordBack + SubscriptionCreate {}

#[cfg(not(feature = "v1"))]
/// trait SubscriptionRecordBack
pub trait SubscriptionRecordBack {}

/// trait Subscriptions
#[cfg(not(feature = "v1"))]
pub trait Subscriptions {}

/// trait SubscriptionCreate
#[cfg(not(feature = "v1"))]
pub trait SubscriptionCreate {}
