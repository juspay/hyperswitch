pub use api_models::payouts::{
    AchBankTransfer, BacsBankTransfer, Bank as BankPayout, Card as CardPayout, PayoutActionRequest,
    PayoutCreateRequest, PayoutCreateResponse, PayoutListConstraints, PayoutListFilterConstraints,
    PayoutListFilters, PayoutListResponse, PayoutMethodData, PayoutRequest, PayoutRetrieveBody,
    PayoutRetrieveRequest, PixBankTransfer, SepaBankTransfer, Wallet as WalletPayout,
};
pub use hyperswitch_domain_models::router_flow_types::payouts::{
    PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient, PoRecipientAccount,
};

use crate::{services::api, types};

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

pub trait PayoutRecipientAccount:
    api::ConnectorIntegration<PoRecipientAccount, types::PayoutsData, types::PayoutsResponseData>
{
}
