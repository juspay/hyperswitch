//! SubscriptionsV2
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::GetSubscriptionPlansData,
    router_flow_types::subscriptions::GetSubscriptionPlans,
    router_request_types::subscriptions::GetSubscriptionPlansRequest,
    router_response_types::subscriptions::GetSubscriptionPlansResponse,
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait SubscriptionsV2
pub trait SubscriptionsV2: GetSubscriptionPlansV2 {}

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
