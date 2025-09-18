//! SubscriptionsV2
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::{GetSubscriptionPlanPricesData, GetSubscriptionPlansData},
    router_flow_types::subscriptions::{GetSubscriptionPlanPrices, GetSubscriptionPlans},
    router_request_types::subscriptions::{
        GetSubscriptionPlanPricesRequest, GetSubscriptionPlansRequest,
    },
    router_response_types::subscriptions::{
        GetSubscriptionPlanPricesResponse, GetSubscriptionPlansResponse,
    },
};

use super::payments_v2::ConnectorCustomerV2;
use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait SubscriptionsV2
pub trait SubscriptionsV2: GetSubscriptionPlansV2 + ConnectorCustomerV2 + GetSubscriptionPlanPricesV2 {}

/// trait GetSubscriptionPlans for V2
pub trait GetSubscriptionPlansV2:
    ConnectorIntegrationV2<
    GetSubscriptionPlans,
    GetSubscriptionPlansData,
    GetSubscriptionPlansRequest,
    GetSubscriptionPlansResponse,
>
{
}

/// trait GetSubscriptionPlans for V2
pub trait GetSubscriptionPlanPricesV2:
    ConnectorIntegrationV2<
    GetSubscriptionPlanPrices,
    GetSubscriptionPlanPricesData,
    GetSubscriptionPlanPricesRequest,
    GetSubscriptionPlanPricesResponse,
>
{
}
