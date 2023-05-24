pub use api_models::payouts::{
    Bank as BankPayout, Card as CardPayout, PayoutActionRequest, PayoutCreateRequest,
    PayoutCreateResponse, PayoutMethodData, PayoutRequest, PayoutRetrieveBody,
    PayoutRetrieveRequest,
};

#[cfg(feature = "payouts")]
use super::ConnectorCommon;
#[cfg(feature = "payouts")]
use crate::{services::api, types};

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PCancel;

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PCreate;

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PEligibility;

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PFulfill;

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PQuote;

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PRecipient;

#[cfg(feature = "payouts")]
pub trait PayoutCancel:
    api::ConnectorIntegration<PCancel, types::PayoutsData, types::PayoutsResponseData>
{
}

#[cfg(feature = "payouts")]
pub trait PayoutCreate:
    api::ConnectorIntegration<PCreate, types::PayoutsData, types::PayoutsResponseData>
{
}

#[cfg(feature = "payouts")]
pub trait PayoutEligibility:
    api::ConnectorIntegration<PEligibility, types::PayoutsData, types::PayoutsResponseData>
{
}

#[cfg(feature = "payouts")]
pub trait PayoutFulfill:
    api::ConnectorIntegration<PFulfill, types::PayoutsData, types::PayoutsResponseData>
{
}

#[cfg(feature = "payouts")]
pub trait PayoutQuote:
    api::ConnectorIntegration<PQuote, types::PayoutsData, types::PayoutsResponseData>
{
}

#[cfg(feature = "payouts")]
pub trait PayoutRecipient:
    api::ConnectorIntegration<PRecipient, types::PayoutsData, types::PayoutsResponseData>
{
}

#[cfg(feature = "payouts")]
pub trait Payouts:
    ConnectorCommon
    + PayoutCancel
    + PayoutCreate
    + PayoutEligibility
    + PayoutFulfill
    + PayoutQuote
    + PayoutRecipient
{
}
#[cfg(not(feature = "payouts"))]
pub trait Payouts {}
