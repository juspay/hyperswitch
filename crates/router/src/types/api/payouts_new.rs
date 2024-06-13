pub use api_models::payouts::{
    AchBankTransfer, BacsBankTransfer, Bank as BankPayout, CardPayout, PayoutActionRequest,
    PayoutCreateRequest, PayoutCreateResponse, PayoutListConstraints, PayoutListFilterConstraints,
    PayoutListFilters, PayoutListResponse, PayoutMethodData, PayoutRequest, PayoutRetrieveBody,
    PayoutRetrieveRequest, PixBankTransfer, SepaBankTransfer, Wallet as WalletPayout,
};
pub use hyperswitch_domain_models::router_flow_types::payouts::{
    PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient, PoRecipientAccount,
};

use crate::{
    services::api,
    types::{self, api as api_types},
};

pub trait PayoutCancelNew:
    api::ConnectorIntegrationNew<
    PoCancel,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutCreateNew:
    api::ConnectorIntegrationNew<
    PoCreate,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutEligibilityNew:
    api::ConnectorIntegrationNew<
    PoEligibility,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutFulfillNew:
    api::ConnectorIntegrationNew<
    PoFulfill,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutQuoteNew:
    api::ConnectorIntegrationNew<
    PoQuote,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutRecipientNew:
    api::ConnectorIntegrationNew<
    PoRecipient,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutRecipientAccountNew:
    api::ConnectorIntegrationNew<
    PoRecipientAccount,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

#[cfg(feature = "payouts")]
pub trait PayoutsNew:
    api_types::ConnectorCommon
    + PayoutCancelNew
    + PayoutCreateNew
    + PayoutEligibilityNew
    + PayoutFulfillNew
    + PayoutQuoteNew
    + PayoutRecipientNew
    + PayoutRecipientAccountNew
{
}
