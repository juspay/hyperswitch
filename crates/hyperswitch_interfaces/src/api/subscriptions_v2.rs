//! SubscriptionsV2
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::GetSubscriptionPlanPricesData,
    router_flow_types::subscriptions::GetSubscriptionPlanPrices,
    router_request_types::subscriptions::GetSubscriptionPlanPricesRequest,
    router_response_types::subscriptions::GetSubscriptionPlanPricesResponse,
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait SubscriptionsV2
pub trait SubscriptionsV2: GetSubscriptionPlanPricesV2 {}

/// trait GetSubscriptionPlans for V1
pub trait GetSubscriptionPlanPricesV2:
    ConnectorIntegrationV2<
    GetSubscriptionPlanPrices,
    GetSubscriptionPlanPricesData,
    GetSubscriptionPlanPricesRequest,
    GetSubscriptionPlanPricesResponse,
>
{
}
