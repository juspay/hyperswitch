//! Subscriptions Interface for V1

use hyperswitch_domain_models::{
    router_flow_types::subscriptions::{
        SubscriptionRecordBack as SubscriptionRecordBackFlow,
        SubscriptionCreate as SubscriptionCreateFlow,
    },
    router_request_types::subscriptions::{SubscriptionsRecordBackRequest, SubscriptionCreateRequest},
    router_response_types::{
        revenue_recovery::RevenueRecoveryRecordBackResponse,
        subscriptions::SubscriptionCreateResponse,
    },
};

use super::{ConnectorCommon, ConnectorIntegration};

#[cfg(feature = "v1")]
/// trait SubscriptionRecordBack for V1
pub trait SubscriptionRecordBack:
    ConnectorIntegration<
        SubscriptionRecordBackFlow,
        SubscriptionsRecordBackRequest,
        RevenueRecoveryRecordBackResponse,
    >
{
}

#[cfg(feature = "v1")]
/// trait SubscriptionCreate for V1
pub trait SubscriptionCreate:
    ConnectorIntegration<
        SubscriptionCreateFlow,
        SubscriptionCreateRequest,
        SubscriptionCreateResponse,
    >
{
}

/// trait Subscriptions 
#[cfg(feature = "v1")]
pub trait Subscriptions:
    ConnectorCommon
    + SubscriptionRecordBack
    + SubscriptionCreate
{
}

#[cfg(not(feature = "v1"))]
/// trait SubscriptionRecordBack (disabled when not V1)
pub trait SubscriptionRecordBack {}

/// trait Subscriptions (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait Subscriptions {}

#[cfg(not(feature = "v1"))]
pub trait SubscriptionCreate {}
