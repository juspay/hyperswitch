//! SubscriptionsV2
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::{GetSubscriptionPlansData, InvoiceRecordBackData},
    router_flow_types::{revenue_recovery::InvoiceRecordBack, subscriptions::GetSubscriptionPlans},
    router_request_types::{
        revenue_recovery::InvoiceRecordBackRequest, subscriptions::GetSubscriptionPlansRequest,
    },
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse, subscriptions::GetSubscriptionPlansResponse,
    },
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait SubscriptionsV2
pub trait SubscriptionsV2: GetSubscriptionPlansV2 + SubscriptionRecordBackV2 {}

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

/// trait GetSubscriptionPlans for V2
pub trait SubscriptionRecordBackV2:
    ConnectorIntegrationV2<
    InvoiceRecordBack,
    InvoiceRecordBackData,
    InvoiceRecordBackRequest,
    InvoiceRecordBackResponse,
>
{
}
