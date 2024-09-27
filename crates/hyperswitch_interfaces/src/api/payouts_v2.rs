//! Payouts V2 interface
use hyperswitch_domain_models::router_flow_types::payouts::{
    PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient, PoRecipientAccount, PoSync,
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::PayoutFlowData, router_request_types::PayoutsData,
    router_response_types::PayoutsResponseData,
};

use crate::{api::ConnectorIntegrationV2, errors};

/// trait PayoutCancelV2
pub trait PayoutCancelV2:
    ConnectorIntegrationV2<
    PoCancel,
    PayoutFlowData,
    PayoutsData,
    PayoutsResponseData,
    Error = errors::ConnectorError,
>
{
}

/// trait PayoutCreateV2
pub trait PayoutCreateV2:
    ConnectorIntegrationV2<
    PoCreate,
    PayoutFlowData,
    PayoutsData,
    PayoutsResponseData,
    Error = errors::ConnectorError,
>
{
}

/// trait PayoutEligibilityV2
pub trait PayoutEligibilityV2:
    ConnectorIntegrationV2<
    PoEligibility,
    PayoutFlowData,
    PayoutsData,
    PayoutsResponseData,
    Error = errors::ConnectorError,
>
{
}

/// trait PayoutFulfillV2
pub trait PayoutFulfillV2:
    ConnectorIntegrationV2<
    PoFulfill,
    PayoutFlowData,
    PayoutsData,
    PayoutsResponseData,
    Error = errors::ConnectorError,
>
{
}

/// trait PayoutQuoteV2
pub trait PayoutQuoteV2:
    ConnectorIntegrationV2<
    PoQuote,
    PayoutFlowData,
    PayoutsData,
    PayoutsResponseData,
    Error = errors::ConnectorError,
>
{
}

/// trait PayoutRecipientV2
pub trait PayoutRecipientV2:
    ConnectorIntegrationV2<
    PoRecipient,
    PayoutFlowData,
    PayoutsData,
    PayoutsResponseData,
    Error = errors::ConnectorError,
>
{
}

/// trait PayoutRecipientAccountV2
pub trait PayoutRecipientAccountV2:
    ConnectorIntegrationV2<
    PoRecipientAccount,
    PayoutFlowData,
    PayoutsData,
    PayoutsResponseData,
    Error = errors::ConnectorError,
>
{
}

/// trait PayoutSyncV2
pub trait PayoutSyncV2:
    ConnectorIntegrationV2<
    PoSync,
    PayoutFlowData,
    PayoutsData,
    PayoutsResponseData,
    Error = errors::ConnectorError,
>
{
}
