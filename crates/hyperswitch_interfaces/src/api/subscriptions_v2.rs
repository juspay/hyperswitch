//! SubscriptionsV2
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::{
        GetSubscriptionEstimateData, GetSubscriptionPlanPricesData, GetSubscriptionPlansData,
        SubscriptionCreateData, SubscriptionCustomerData,
    },
    router_flow_types::{
        subscriptions::{GetSubscriptionPlanPrices, GetSubscriptionPlans, SubscriptionCreate},
        CreateConnectorCustomer, GetSubscriptionEstimate,
    },
    router_request_types::{
        subscriptions::{
            GetSubscriptionEstimateRequest, GetSubscriptionPlanPricesRequest,
            GetSubscriptionPlansRequest, SubscriptionCreateRequest,
        },
        ConnectorCustomerData,
    },
    router_response_types::{
        subscriptions::{
            GetSubscriptionEstimateResponse, GetSubscriptionPlanPricesResponse,
            GetSubscriptionPlansResponse, SubscriptionCreateResponse,
        },
        PaymentsResponseData,
    },
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait SubscriptionsV2
pub trait SubscriptionsV2:
    GetSubscriptionPlansV2
    + SubscriptionsCreateV2
    + SubscriptionConnectorCustomerV2
    + GetSubscriptionPlanPricesV2
    + GetSubscriptionEstimateV2
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

/// trait SubscriptionConnectorCustomerV2
pub trait SubscriptionConnectorCustomerV2:
    ConnectorIntegrationV2<
    CreateConnectorCustomer,
    SubscriptionCustomerData,
    ConnectorCustomerData,
    PaymentsResponseData,
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
