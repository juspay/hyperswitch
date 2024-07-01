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

pub trait PayoutCancelV2:
    api::ConnectorIntegrationV2<
    PoCancel,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutCreateV2:
    api::ConnectorIntegrationV2<
    PoCreate,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutEligibilityV2:
    api::ConnectorIntegrationV2<
    PoEligibility,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutFulfillV2:
    api::ConnectorIntegrationV2<
    PoFulfill,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutQuoteV2:
    api::ConnectorIntegrationV2<
    PoQuote,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutRecipientV2:
    api::ConnectorIntegrationV2<
    PoRecipient,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutRecipientAccountV2:
    api::ConnectorIntegrationV2<
    PoRecipientAccount,
    types::PayoutFlowData,
    types::PayoutsData,
    types::PayoutsResponseData,
>
{
}

pub trait PayoutsV2:
    api_types::ConnectorCommon
    + PayoutCancelV2
    + PayoutCreateV2
    + PayoutEligibilityV2
    + PayoutFulfillV2
    + PayoutQuoteV2
    + PayoutRecipientV2
    + PayoutRecipientAccountV2
{
}
