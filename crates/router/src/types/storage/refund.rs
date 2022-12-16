pub use storage_models::refund::{
    Refund, RefundCoreWorkflow, RefundNew, RefundUpdate, RefundUpdateInternal,
};

use crate::utils::storage_partitioning::KvStorePartition;

#[cfg(feature = "kv_store")]
impl KvStorePartition for Refund {}
