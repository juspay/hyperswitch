use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::{RevenueRecoveryRecordBackData, SubscriptionCreateData},
    router_flow_types::{SubscriptionCreate, SubscriptionRecordBack},
    router_request_types::subscriptions::{
        SubscriptionCreateRequest, SubscriptionsRecordBackRequest,
    },
    router_response_types::{
        revenue_recovery::RevenueRecoveryRecordBackResponse,
        subscriptions::SubscriptionCreateResponse,
    },
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait SubscriptionsV2
pub trait SubscriptionsV2: SubscriptionsRecordBackV2 + SubscriptionsCreateV2 {}

/// trait SubscriptionsRecordBackV2
pub trait SubscriptionsRecordBackV2:
    ConnectorIntegrationV2<
    SubscriptionRecordBack,
    RevenueRecoveryRecordBackData,
    SubscriptionsRecordBackRequest,
    RevenueRecoveryRecordBackResponse,
>
{
}

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
