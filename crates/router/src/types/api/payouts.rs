pub use api_models::payouts::{
    Bank as BankPayout, Card as CardPayout, PayoutActionRequest, PayoutCreateRequest,
    PayoutCreateResponse, PayoutMethodData, PayoutRequest, PayoutRetrieveBody,
    PayoutRetrieveRequest,
};

use super::ConnectorCommon;
use crate::{services::api, types};

#[derive(Debug, Clone)]
pub struct PCreate;

#[derive(Debug, Clone)]
pub struct PEligibility;

#[derive(Debug, Clone)]
pub struct PFulfill;

pub trait PayoutCreate:
    api::ConnectorIntegration<PCreate, types::PayoutsData, types::PayoutsResponseData>
{
}

pub trait PayoutEligibility:
    api::ConnectorIntegration<PEligibility, types::PayoutsData, types::PayoutsResponseData>
{
}

pub trait PayoutFulfill:
    api::ConnectorIntegration<PFulfill, types::PayoutsData, types::PayoutsResponseData>
{
}

pub trait Payouts: ConnectorCommon + PayoutCreate + PayoutEligibility + PayoutFulfill {}
