//! Subscriptions Interface for V1

use hyperswitch_domain_models::{
    router_flow_types::{
        subscriptions::{
            GetSubscriptionEstimate, GetSubscriptionItemPrices, GetSubscriptionItems,
            SubscriptionCreate as SubscriptionCreateFlow,
        },
        InvoiceRecordBack,
    },
    router_request_types::{
        revenue_recovery::InvoiceRecordBackRequest,
        subscriptions::{
            GetSubscriptionEstimateRequest, GetSubscriptionItemPricesRequest,
            GetSubscriptionItemsRequest, SubscriptionCreateRequest,
        },
    },
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse,
        subscriptions::{
            GetSubscriptionEstimateResponse, GetSubscriptionItemPricesResponse,
            GetSubscriptionItemsResponse, SubscriptionCreateResponse,
        },
    },
};

use super::{
    payments::ConnectorCustomer as PaymentsConnectorCustomer, ConnectorCommon, ConnectorIntegration,
};

/// trait GetSubscriptionItems for V1
pub trait GetSubscriptionItemsFlow:
    ConnectorIntegration<
    GetSubscriptionItems,
    GetSubscriptionItemsRequest,
    GetSubscriptionItemsResponse,
>
{
}

/// trait SubscriptionRecordBack for V1
pub trait SubscriptionRecordBackFlow:
    ConnectorIntegration<InvoiceRecordBack, InvoiceRecordBackRequest, InvoiceRecordBackResponse>
{
}

/// trait SubscriptionPause for V1
pub trait SubscriptionPauseFlow:
    ConnectorIntegration<
    hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionPause,
    hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionPauseRequest,
    hyperswitch_domain_models::router_response_types::subscriptions::SubscriptionPauseResponse,
>
{
}

/// trait SubscriptionResume for V1
pub trait SubscriptionResumeFlow:
    ConnectorIntegration<
    hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionResume,
    hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionResumeRequest,
    hyperswitch_domain_models::router_response_types::subscriptions::SubscriptionResumeResponse,
>
{
}

/// trait SubscriptionCancel for V1
pub trait SubscriptionCancelFlow:
    ConnectorIntegration<
    hyperswitch_domain_models::router_flow_types::subscriptions::SubscriptionCancel,
    hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionCancelRequest,
    hyperswitch_domain_models::router_response_types::subscriptions::SubscriptionCancelResponse,
>
{
}

/// trait GetSubscriptionItemPrices for V1
pub trait GetSubscriptionPlanPricesFlow:
    ConnectorIntegration<
    GetSubscriptionItemPrices,
    GetSubscriptionItemPricesRequest,
    GetSubscriptionItemPricesResponse,
>
{
}

/// trait SubscriptionCreate
pub trait SubscriptionCreate:
    ConnectorIntegration<SubscriptionCreateFlow, SubscriptionCreateRequest, SubscriptionCreateResponse>
{
}

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
pub trait Subscriptions:
    ConnectorCommon
    + GetSubscriptionItemsFlow
    + GetSubscriptionPlanPricesFlow
    + SubscriptionCreate
    + PaymentsConnectorCustomer
    + SubscriptionRecordBackFlow
    + GetSubscriptionEstimateFlow
    + SubscriptionPauseFlow
    + SubscriptionResumeFlow
    + SubscriptionCancelFlow
{
}
