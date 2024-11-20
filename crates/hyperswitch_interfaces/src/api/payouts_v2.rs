//! Payouts V2 interface
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::PayoutFlowData,
    router_flow_types::payouts::{
        PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient, PoRecipientAccount,
        PoSync,
    },
    router_request_types::PayoutsData,
    router_response_types::PayoutsResponseData,
};

use super::ConnectorCommon;
use crate::api::ConnectorIntegrationV2;

/// trait PayoutCancelV2
pub trait PayoutCancelV2:
    ConnectorIntegrationV2<PoCancel, PayoutFlowData, PayoutsData, PayoutsResponseData>
{
}

/// trait PayoutCreateV2
pub trait PayoutCreateV2:
    ConnectorIntegrationV2<PoCreate, PayoutFlowData, PayoutsData, PayoutsResponseData>
{
}

/// trait PayoutEligibilityV2
pub trait PayoutEligibilityV2:
    ConnectorIntegrationV2<PoEligibility, PayoutFlowData, PayoutsData, PayoutsResponseData>
{
}

/// trait PayoutFulfillV2
pub trait PayoutFulfillV2:
    ConnectorIntegrationV2<PoFulfill, PayoutFlowData, PayoutsData, PayoutsResponseData>
{
}

/// trait PayoutQuoteV2
pub trait PayoutQuoteV2:
    ConnectorIntegrationV2<PoQuote, PayoutFlowData, PayoutsData, PayoutsResponseData>
{
}

/// trait PayoutRecipientV2
pub trait PayoutRecipientV2:
    ConnectorIntegrationV2<PoRecipient, PayoutFlowData, PayoutsData, PayoutsResponseData>
{
}

/// trait PayoutRecipientAccountV2
pub trait PayoutRecipientAccountV2:
    ConnectorIntegrationV2<PoRecipientAccount, PayoutFlowData, PayoutsData, PayoutsResponseData>
{
}

/// trait PayoutSyncV2
pub trait PayoutSyncV2:
    ConnectorIntegrationV2<PoSync, PayoutFlowData, PayoutsData, PayoutsResponseData>
{
}

#[cfg(feature = "payouts")]
/// trait Payouts
pub trait PayoutsV2:
    ConnectorCommon
    + PayoutCancelV2
    + PayoutCreateV2
    + PayoutEligibilityV2
    + PayoutFulfillV2
    + PayoutQuoteV2
    + PayoutRecipientV2
    + PayoutRecipientAccountV2
    + PayoutSyncV2
{
}

/// Empty trait for when payouts feature is disabled
#[cfg(not(feature = "payouts"))]
pub trait PayoutsV2 {}
