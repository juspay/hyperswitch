use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::SubscriptionCreateData,
    router_flow_types::SubscriptionCreate,
    router_request_types::subscriptions::SubscriptionCreateRequest,
    router_response_types::subscriptions::SubscriptionCreateResponse,
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait SubscriptionsV2
pub trait SubscriptionsV2: SubscriptionsCreateV2 {}

/// trait SubscriptionsCreateV2
pub trait SubscriptionsCreateV2:
    ConnectorIntegrationV2<
    SubscriptionCreate,
    SubscriptionCreateData,
    SubscriptionCreateRequest,
    SubscriptionCreateResponse,
>
{
}
