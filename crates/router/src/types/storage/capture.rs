pub use storage_models::capture::*;

#[cfg(feature = "kv_store")]
impl crate::utils::storage_partitioning::KvStorePartition for Capture {}
