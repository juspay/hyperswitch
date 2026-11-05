//! API sender identity, persisted on the payout and normalized into common payout
//! vendor details at the domain boundary.
//!
//! The definitions live in `common_types::payouts` so the storage models can persist
//! the sender identity directly; this module re-exports them for API consumers.
pub use common_types::payouts::{
    CorporateSenderDetails, IndividualSenderDetails, SenderAddress, SenderDetails, SenderPhone,
    SourceOfFunds,
};
