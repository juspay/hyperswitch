//! SubscriptionsV2
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::{GetSubscriptionEstimateData, GetSubscriptionPlansData},
    router_flow_types::subscriptions::{GetSubscriptionEstimate, GetSubscriptionPlans},
    router_request_types::subscriptions::{
        GetSubscriptionEstimateRequest, GetSubscriptionPlansRequest,
    },
    router_response_types::subscriptions::{
        GetSubscriptionEstimateResponse, GetSubscriptionPlansResponse,
    },
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait SubscriptionsV2
pub trait SubscriptionsV2: GetSubscriptionPlansV2 + GetSubscriptionEstimateV2 {}

/// trait GetSubscriptionPlans for V1
pub trait GetSubscriptionPlansV2:
    ConnectorIntegrationV2<
    GetSubscriptionPlans,
    GetSubscriptionPlansData,
    GetSubscriptionPlansRequest,
    GetSubscriptionPlansResponse,
>
{
}

/// trait GetSubscriptionEstimate for V2
pub trait GetSubscriptionEstimateV2:
    ConnectorIntegrationV2<
    GetSubscriptionEstimate,
    GetSubscriptionEstimateData,
    GetSubscriptionEstimateRequest,
    GetSubscriptionEstimateResponse,
>
{
}
