//! SubscriptionsV2
use hyperswitch_domain_models::{
    router_flow_types::{
        revenue_recovery::InvoiceRecordBack,
        subscriptions::{GetSubscriptionPlanPrices, GetSubscriptionPlans, SubscriptionCreate},
    },
    router_request_types::subscriptions::{
        GetSubscriptionPlanPricesRequest, GetSubscriptionPlansRequest, SubscriptionCreateRequest,
    },
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse,
        subscriptions::{
            GetSubscriptionPlanPricesResponse, GetSubscriptionPlansResponse,
            SubscriptionCreateResponse,
        },
    },
};

use super::payments_v2::ConnectorCustomerV2;
use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait SubscriptionsV2
pub trait SubscriptionsV2:
    GetSubscriptionPlansV2
    + SubscriptionsCreateV2
    + ConnectorCustomerV2
    + GetSubscriptionPlanPricesV2
    + SubscriptionRecordBackV2
{
}

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
pub trait SubscriptionRecordBackV2:
    ConnectorIntegrationV2<
    InvoiceRecordBack,
    InvoiceRecordBackData,
    InvoiceRecordBackRequest,
    InvoiceRecordBackResponse,
>
{
}
pub trait GetSubscriptionPlanPricesV2:
    ConnectorIntegrationV2<
    GetSubscriptionPlanPrices,
    GetSubscriptionPlanPricesData,
    GetSubscriptionPlanPricesRequest,
    GetSubscriptionPlanPricesResponse,
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
