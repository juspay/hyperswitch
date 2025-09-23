//! Subscriptions Interface for V1
#[cfg(feature = "v1")]
use hyperswitch_domain_models::{
    router_flow_types::subscriptions::{GetSubscriptionPlanPrices, GetSubscriptionPlans},
    router_flow_types::{
        subscriptions::SubscriptionCreate as SubscriptionCreateFlow, InvoiceRecordBack,
    },
    router_request_types::{
        revenue_recovery::InvoiceRecordBackRequest,
        subscriptions::{
            GetSubscriptionPlanPricesRequest, GetSubscriptionPlansRequest,
            SubscriptionCreateRequest,
        },
    },
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse,
        subscriptions::{
            GetSubscriptionPlanPricesResponse, GetSubscriptionPlansResponse,
            SubscriptionCreateResponse,
        },
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
/// trait GetSubscriptionPlans for V1
pub trait SubscriptionRecordBackFlow:
    ConnectorIntegration<InvoiceRecordBack, InvoiceRecordBackRequest, InvoiceRecordBackResponse>
{
}
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

/// trait Subscriptions
#[cfg(feature = "v1")]
pub trait Subscriptions:
    ConnectorCommon
    + GetSubscriptionPlansFlow
    + GetSubscriptionPlanPricesFlow
    + SubscriptionCreate
    + PaymentsConnectorCustomer
    + SubscriptionRecordBackFlow
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
