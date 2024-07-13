pub mod payout_attempt;
#[allow(clippy::module_inception)]
pub mod payouts;

use diesel_models::{payout_attempt::PayoutAttempt, payouts::Payouts};

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for Payouts {}
impl KvStorePartition for PayoutAttempt {}
