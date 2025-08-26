//! Subscriptions Interface for V1

use hyperswitch_domain_models::{
    router_flow_types::subscriptions::SubscriptionRecordBack as SubscriptionRecordBackFlow,
    router_request_types::revenue_recovery::RevenueRecoveryRecordBackRequest,
    router_response_types::revenue_recovery::RevenueRecoveryRecordBackResponse,
};

use super::{ConnectorCommon, ConnectorIntegration};

/// trait Subscriptions for V1
#[cfg(feature = "v1")]
pub trait Subscriptions: ConnectorCommon + SubscriptionRecordBack {}

/// trait SubscriptionRecordBack for V1
#[cfg(feature = "v1")]
pub trait SubscriptionRecordBack:
    ConnectorIntegration<
        SubscriptionRecordBackFlow,
        RevenueRecoveryRecordBackRequest,
        RevenueRecoveryRecordBackResponse,
    >
{
}

#[cfg(not(feature = "v1"))]
/// trait Subscriptions (disabled when not V1)
pub trait Subscriptions {}

#[cfg(not(feature = "v1"))]
/// trait SubscriptionRecordBack (disabled when not V1)
pub trait SubscriptionRecordBack {}
