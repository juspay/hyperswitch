//! Subscriptions Interface for V1
#[cfg(feature = "v1")]
use hyperswitch_domain_models::{
    router_flow_types::subscriptions::SubscriptionCreate as SubscriptionCreateFlow,
    router_flow_types::subscriptions::{
        GetSubscriptionEstimate, GetSubscriptionPlanPrices, GetSubscriptionPlans,
    },
    router_request_types::subscriptions::{
        GetSubscriptionEstimateRequest, GetSubscriptionPlanPricesRequest,
        GetSubscriptionPlansRequest, SubscriptionCreateRequest,
    },
    router_response_types::subscriptions::{
        GetSubscriptionEstimateResponse, GetSubscriptionPlanPricesResponse,
        GetSubscriptionPlansResponse, SubscriptionCreateResponse,
    },
};

#[cfg(feature = "v1")]
use super::{
    payments::ConnectorCustomer as PaymentsConnectorCustomer, ConnectorCommon, ConnectorIntegration,
};

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
/// trait GetSubscriptionPlanPrices for V1
pub trait GetSubscriptionPlanPricesFlow:
    ConnectorIntegration<
    GetSubscriptionPlanPrices,
    GetSubscriptionPlanPricesRequest,
    GetSubscriptionPlanPricesResponse,
>
{
}

#[cfg(feature = "v1")]
/// trait SubscriptionCreate
pub trait SubscriptionCreate:
    ConnectorIntegration<SubscriptionCreateFlow, SubscriptionCreateRequest, SubscriptionCreateResponse>
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
pub trait Subscriptions:
    ConnectorCommon
    + GetSubscriptionPlansFlow
    + GetSubscriptionPlanPricesFlow
    + SubscriptionCreate
    + PaymentsConnectorCustomer
    + GetSubscriptionEstimateFlow
{
}

/// trait Subscriptions (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait Subscriptions {}

/// trait GetSubscriptionPlansFlow (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait GetSubscriptionPlansFlow {}

/// trait GetSubscriptionPlanPricesFlow (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait GetSubscriptionPlanPricesFlow {}

#[cfg(not(feature = "v1"))]
/// trait CreateCustomer (disabled when not V1)
pub trait ConnectorCustomer {}

/// trait SubscriptionCreate
#[cfg(not(feature = "v1"))]
pub trait SubscriptionCreate {}

/// trait GetSubscriptionEstimateFlow (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait GetSubscriptionEstimateFlow {}
