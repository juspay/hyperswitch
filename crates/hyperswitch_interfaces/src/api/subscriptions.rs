use super::{ConnectorCommon, ConnectorIntegration};

#[cfg(feature = "v1")]
/// trait CustomerCreate for V1
pub trait CreateCustomer:
    ConnectorIntegration<CreateCustomerFlow, CreateCustomerRequest, CreateCustomerResponse>
{
}

#[cfg(feature = "v1")]
use hyperswitch_domain_models::{
    router_flow_types::subscriptions::{
        CreateCustomer as CreateCustomerFlow, SubscriptionCreate as SubscriptionCreateFlow,
        SubscriptionRecordBack as SubscriptionRecordBackFlow,
    },
    router_request_types::subscriptions::{
        CreateCustomerRequest, SubscriptionCreateRequest, SubscriptionsRecordBackRequest,
    },
    router_response_types::{
        revenue_recovery::RevenueRecoveryRecordBackResponse,
        subscriptions::{CreateCustomerResponse, SubscriptionCreateResponse},
    },
};
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

#[cfg(not(feature = "v1"))]
/// trait CreateCustomer (disabled when not V1)
pub trait CreateCustomer {}

#[cfg(not(feature = "v1"))]
/// trait Subscriptions (disabled when not V1)
pub trait Subscriptions {}
/// trait SubscriptionCreate
pub trait SubscriptionCreate:
    ConnectorIntegration<SubscriptionCreateFlow, SubscriptionCreateRequest, SubscriptionCreateResponse>
{
}

/// trait Subscriptions
#[cfg(feature = "v1")]
pub trait Subscriptions:
    ConnectorCommon + SubscriptionRecordBack + SubscriptionCreate + CreateCustomer
{
}

#[cfg(not(feature = "v1"))]
/// trait SubscriptionRecordBack
pub trait SubscriptionRecordBack {}

/// trait Subscriptions
#[cfg(not(feature = "v1"))]
pub trait Subscriptions {}

/// trait SubscriptionCreate
#[cfg(not(feature = "v1"))]
pub trait SubscriptionCreate {}
