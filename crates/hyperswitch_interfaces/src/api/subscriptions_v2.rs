//! SubscriptionsV2
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::{
        RevenueRecoveryRecordBackData,
    },
    router_flow_types::{
        SubscriptionRecordBack,
    },
    router_request_types::revenue_recovery::{
        RevenueRecoveryRecordBackRequest,
    },
    router_response_types::revenue_recovery::{
        RevenueRecoveryRecordBackResponse,
    },
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

pub trait SubscriptionsV2: SubscriptionsRecordBackV2 {}


pub trait SubscriptionsRecordBackV2:
    ConnectorIntegrationV2<
    SubscriptionRecordBack,
    RevenueRecoveryRecordBackData,
    RevenueRecoveryRecordBackRequest,
    RevenueRecoveryRecordBackResponse,
>
{
}