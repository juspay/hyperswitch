pub use api_models::payouts::{
    AchBankTransfer, BacsBankTransfer, Bank as BankPayout, CardPayout, PayoutActionRequest,
    PayoutAttemptResponse, PayoutCreateRequest, PayoutCreateResponse, PayoutListConstraints,
    PayoutListFilterConstraints, PayoutListFilters, PayoutListResponse, PayoutMethodData,
    PayoutRequest, PayoutRetrieveBody, PayoutRetrieveRequest, PixBankTransfer, SepaBankTransfer,
    Wallet as WalletPayout,
};
pub use hyperswitch_domain_models::router_flow_types::payouts::{
    PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient, PoRecipientAccount, PoSync,
};
pub use hyperswitch_interfaces::api::payouts_v2::{
    PayoutCancelV2, PayoutCreateV2, PayoutEligibilityV2, PayoutFulfillV2, PayoutQuoteV2,
    PayoutRecipientAccountV2, PayoutRecipientV2, PayoutSyncV2,
};

use crate::types::api as api_types;

pub trait PayoutsV2:
    api_types::ConnectorCommon
    + PayoutCancelV2
    + PayoutCreateV2
    + PayoutEligibilityV2
    + PayoutFulfillV2
    + PayoutQuoteV2
    + PayoutRecipientV2
    + PayoutSyncV2
    + PayoutRecipientAccountV2
{
}
