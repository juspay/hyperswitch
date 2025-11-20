//! Subscriptions Interface for V1

use hyperswitch_domain_models::{
    router_flow_types::{
        subscriptions::{
            GetSubscriptionEstimate, GetSubscriptionPlanPrices, GetSubscriptionPlans,
            SubscriptionCreate as SubscriptionCreateFlow,
        },
        InvoiceRecordBack,
    },
    router_request_types::{
        revenue_recovery::InvoiceRecordBackRequest,
        subscriptions::{
            GetSubscriptionEstimateRequest, GetSubscriptionPlanPricesRequest,
            GetSubscriptionPlansRequest, SubscriptionCreateRequest,
        },
    },
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse,
        subscriptions::{
            GetSubscriptionEstimateResponse, GetSubscriptionPlanPricesResponse,
            GetSubscriptionPlansResponse, SubscriptionCreateResponse,
        },
    },
};

use super::{
    payments::ConnectorCustomer as PaymentsConnectorCustomer, ConnectorCommon, ConnectorIntegration,
};

/// trait GetSubscriptionPlans for V1
pub trait GetSubscriptionPlansFlow:
    ConnectorIntegration<
    GetSubscriptionPlans,
    GetSubscriptionPlansRequest,
    GetSubscriptionPlansResponse,
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

/// trait GetSubscriptionPlanPrices for V1
pub trait GetSubscriptionPlanPricesFlow:
    ConnectorIntegration<
    GetSubscriptionPlanPrices,
    GetSubscriptionPlanPricesRequest,
    GetSubscriptionPlanPricesResponse,
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
    + GetSubscriptionPlansFlow
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
