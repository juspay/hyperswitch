pub use api_models::payouts::{
    Bank as BankPayout, Card as CardPayout, PayoutActionRequest, PayoutCreateRequest,
    PayoutCreateResponse, PayoutMethodData, PayoutRequest, PayoutRetrieveBody,
    PayoutRetrieveRequest,
};

use super::ConnectorCommon;
use crate::{services::api, types};

#[derive(Debug, Clone)]
pub struct PCancel;

#[derive(Debug, Clone)]
pub struct PCreate;

#[derive(Debug, Clone)]
pub struct PEligibility;

#[derive(Debug, Clone)]
pub struct PFulfill;

#[derive(Debug, Clone)]
pub struct PQuote;

#[derive(Debug, Clone)]
pub struct PRecipient;

pub trait PayoutCancel:
    api::ConnectorIntegration<PCancel, types::PayoutsData, types::PayoutsResponseData>
{
}

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

pub trait PayoutQuote:
    api::ConnectorIntegration<PQuote, types::PayoutsData, types::PayoutsResponseData>
{
}

pub trait PayoutRecipient:
    api::ConnectorIntegration<PRecipient, types::PayoutsData, types::PayoutsResponseData>
{
}

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
