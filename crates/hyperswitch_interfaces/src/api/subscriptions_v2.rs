//! SubscriptionsV2
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::{
        GetSubscriptionEstimateData, GetSubscriptionItemPricesData, GetSubscriptionItemsData,
        InvoiceRecordBackData, SubscriptionCancelData, SubscriptionCreateData,
        SubscriptionCustomerData, SubscriptionPauseData, SubscriptionResumeData,
    },
    router_flow_types::{
        revenue_recovery::InvoiceRecordBack,
        subscriptions::{
            GetSubscriptionEstimate, GetSubscriptionItemPrices, GetSubscriptionItems,
            SubscriptionCancel, SubscriptionCreate, SubscriptionPause, SubscriptionResume,
        },
        CreateConnectorCustomer,
    },
    router_request_types::{
        revenue_recovery::InvoiceRecordBackRequest,
        subscriptions::{
            GetSubscriptionEstimateRequest, GetSubscriptionItemPricesRequest,
            GetSubscriptionItemsRequest, SubscriptionCancelRequest, SubscriptionCreateRequest,
            SubscriptionPauseRequest, SubscriptionResumeRequest,
        },
        ConnectorCustomerData,
    },
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse,
        subscriptions::{
            GetSubscriptionEstimateResponse, GetSubscriptionItemPricesResponse,
            GetSubscriptionItemsResponse, SubscriptionCancelResponse, SubscriptionCreateResponse,
            SubscriptionPauseResponse, SubscriptionResumeResponse,
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
    + SubscriptionRecordBackV2
    + GetSubscriptionEstimateV2
    + SubscriptionCancelV2
    + SubscriptionPauseV2
    + SubscriptionResumeV2
{
}

/// trait GetSubscriptionItems for V2
pub trait GetSubscriptionPlansV2:
    ConnectorIntegrationV2<
    GetSubscriptionItems,
    GetSubscriptionItemsData,
    GetSubscriptionItemsRequest,
    GetSubscriptionItemsResponse,
>
{
}

/// trait SubscriptionRecordBack for V2
pub trait SubscriptionRecordBackV2:
    ConnectorIntegrationV2<
    InvoiceRecordBack,
    InvoiceRecordBackData,
    InvoiceRecordBackRequest,
    InvoiceRecordBackResponse,
>
{
}
/// trait GetSubscriptionPlanPricesV2 for V2
pub trait GetSubscriptionPlanPricesV2:
    ConnectorIntegrationV2<
    GetSubscriptionItemPrices,
    GetSubscriptionItemPricesData,
    GetSubscriptionItemPricesRequest,
    GetSubscriptionItemPricesResponse,
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

/// trait SubscriptionCancel for V2
pub trait SubscriptionCancelV2:
    ConnectorIntegrationV2<
    SubscriptionCancel,
    SubscriptionCancelData,
    SubscriptionCancelRequest,
    SubscriptionCancelResponse,
>
{
}

/// trait SubscriptionPause for V2
pub trait SubscriptionPauseV2:
    ConnectorIntegrationV2<
    SubscriptionPause,
    SubscriptionPauseData,
    SubscriptionPauseRequest,
    SubscriptionPauseResponse,
>
{
}
/// trait SubscriptionResume for V2
pub trait SubscriptionResumeV2:
    ConnectorIntegrationV2<
    SubscriptionResume,
    SubscriptionResumeData,
    SubscriptionResumeRequest,
    SubscriptionResumeResponse,
>
{
}
