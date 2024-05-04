pub use api_models::payouts::{
    AchBankTransfer, BacsBankTransfer, Bank as BankPayout, Card as CardPayout, PayoutActionRequest,
    PayoutCreateRequest, PayoutCreateResponse, PayoutListConstraints, PayoutListFilterConstraints,
    PayoutListFilters, PayoutListResponse, PayoutMethodData, PayoutRequest, PayoutRetrieveBody,
    PayoutRetrieveRequest, SepaBankTransfer, Wallet as WalletPayout,
};

use crate::{services::api, types};

#[derive(Debug, Clone)]
pub struct PoCancel;

#[derive(Debug, Clone)]
pub struct PoCreate;

#[derive(Debug, Clone)]
pub struct PoEligibility;

#[derive(Debug, Clone)]
pub struct PoFulfill;

#[derive(Debug, Clone)]
pub struct PoQuote;

#[derive(Debug, Clone)]
pub struct PoRecipient;

pub trait PayoutCancel:
    api::ConnectorIntegration<PoCancel, types::PayoutsData, types::PayoutsResponseData>
{
}

pub trait PayoutCreate:
    api::ConnectorIntegration<PoCreate, types::PayoutsData, types::PayoutsResponseData>
{
}

pub trait PayoutEligibility:
    api::ConnectorIntegration<PoEligibility, types::PayoutsData, types::PayoutsResponseData>
{
}

pub trait PayoutFulfill:
    api::ConnectorIntegration<PoFulfill, types::PayoutsData, types::PayoutsResponseData>
{
}

pub trait PayoutQuote:
    api::ConnectorIntegration<PoQuote, types::PayoutsData, types::PayoutsResponseData>
{
}

pub trait PayoutRecipient:
    api::ConnectorIntegration<PoRecipient, types::PayoutsData, types::PayoutsResponseData>
{
}
