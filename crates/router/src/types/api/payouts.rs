pub use api_models::payouts::{
    AchBankTransfer, BacsBankTransfer, Bank as BankPayout, Card as CardPayout, PayoutActionRequest,
    PayoutCreateRequest, PayoutCreateResponse, PayoutMethodData, PayoutRequest, PayoutRetrieveBody,
    PayoutRetrieveRequest, SepaBankTransfer, Wallet as WalletPayout,
};

#[cfg(feature = "payouts")]
use super::ConnectorCommon;
#[cfg(feature = "payouts")]
use crate::{services::api, types};

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PoCancel;

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PoCreate;

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PoEligibility;

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PoFulfill;

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PoQuote;

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PoRecipient;

#[cfg(feature = "payouts")]
pub trait PayoutCancel:
    api::ConnectorIntegration<PoCancel, types::PayoutsData, types::PayoutsResponseData>
{
}

#[cfg(feature = "payouts")]
pub trait PayoutCreate:
    api::ConnectorIntegration<PoCreate, types::PayoutsData, types::PayoutsResponseData>
{
}

#[cfg(feature = "payouts")]
pub trait PayoutEligibility:
    api::ConnectorIntegration<PoEligibility, types::PayoutsData, types::PayoutsResponseData>
{
}

#[cfg(feature = "payouts")]
pub trait PayoutFulfill:
    api::ConnectorIntegration<PoFulfill, types::PayoutsData, types::PayoutsResponseData>
{
}

#[cfg(feature = "payouts")]
pub trait PayoutQuote:
    api::ConnectorIntegration<PoQuote, types::PayoutsData, types::PayoutsResponseData>
{
}

#[cfg(feature = "payouts")]
pub trait PayoutRecipient:
    api::ConnectorIntegration<PoRecipient, types::PayoutsData, types::PayoutsResponseData>
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
