//! Subscriptions Interface for V1
#[cfg(feature = "v1")]
use hyperswitch_domain_models::{
    router_flow_types::{subscriptions::GetSubscriptionPlans, InvoiceRecordBack},
    router_request_types::{
        revenue_recovery::InvoiceRecordBackRequest, subscriptions::GetSubscriptionPlansRequest,
    },
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse, subscriptions::GetSubscriptionPlansResponse,
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

/// trait Subscriptions
#[cfg(feature = "v1")]
pub trait Subscriptions:
    ConnectorCommon + GetSubscriptionPlansFlow + SubscriptionRecordBackFlow + PaymentsConnectorCustomer
{
}

/// trait Subscriptions (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait Subscriptions {}

/// trait GetSubscriptionPlansFlow (disabled when not V1)
#[cfg(not(feature = "v1"))]
pub trait GetSubscriptionPlansFlow {}

#[cfg(not(feature = "v1"))]
/// trait CreateCustomer (disabled when not V1)
pub trait ConnectorCustomer {}
