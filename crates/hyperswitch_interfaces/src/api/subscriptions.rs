//! Subscriptions Interface for V1
#[cfg(feature = "v1")]
use hyperswitch_domain_models::{
    router_flow_types::subscriptions::{GetSubscriptionPlans, GetSubscriptionEstimate},
    router_request_types::subscriptions::{GetSubscriptionPlansRequest, GetSubscriptionEstimateRequest},
    router_response_types::subscriptions::{GetSubscriptionPlansResponse, GetSubscriptionEstimateResponse},
};

#[cfg(feature = "v1")]
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

#[cfg(feature = "v1")]
/// trait GetSubscriptionEstimate for V1
pub trait GetSubscriptionEstimateFlow:
    ConnectorIntegration<
    GetSubscriptionEstimate,
    GetSubscriptionEstimateRequest,
    GetSubscriptionEstimateResponse,
>
{
}

/// trait Subscriptions
#[cfg(feature = "v1")]
pub trait Subscriptions: ConnectorCommon + GetSubscriptionPlansFlow + GetSubscriptionEstimateFlow {}

/// trait Subscriptions (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait Subscriptions {}

/// trait GetSubscriptionPlansFlow (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait GetSubscriptionPlansFlow {}

/// trait GetSubscriptionEstimateFlow (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait GetSubscriptionEstimateFlow {}
