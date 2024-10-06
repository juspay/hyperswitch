//! Payouts interface

use hyperswitch_domain_models::router_flow_types::payouts::{
    PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient, PoRecipientAccount, PoSync,
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_request_types::PayoutsData, router_response_types::PayoutsResponseData,
};

use crate::api::ConnectorIntegration;

/// trait PayoutCancel
pub trait PayoutCancel: ConnectorIntegration<PoCancel, PayoutsData, PayoutsResponseData> {}

/// trait PayoutCreate
pub trait PayoutCreate: ConnectorIntegration<PoCreate, PayoutsData, PayoutsResponseData> {}

/// trait PayoutEligibility
pub trait PayoutEligibility:
    ConnectorIntegration<PoEligibility, PayoutsData, PayoutsResponseData>
{
}

/// trait PayoutFulfill
pub trait PayoutFulfill: ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> {}

/// trait PayoutQuote
pub trait PayoutQuote: ConnectorIntegration<PoQuote, PayoutsData, PayoutsResponseData> {}

/// trait PayoutRecipient
pub trait PayoutRecipient:
    ConnectorIntegration<PoRecipient, PayoutsData, PayoutsResponseData>
{
}

/// trait PayoutRecipientAccount
pub trait PayoutRecipientAccount:
    ConnectorIntegration<PoRecipientAccount, PayoutsData, PayoutsResponseData>
{
}

/// trait PayoutSync
pub trait PayoutSync: ConnectorIntegration<PoSync, PayoutsData, PayoutsResponseData> {}
