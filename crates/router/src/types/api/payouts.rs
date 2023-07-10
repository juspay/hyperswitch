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
pub struct PCreate;

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PEligibility;

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PFulfill;

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
pub trait Payouts: ConnectorCommon + PayoutCreate + PayoutEligibility + PayoutFulfill {}

#[cfg(not(feature = "payouts"))]
pub trait Payouts {}
