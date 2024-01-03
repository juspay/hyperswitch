use diesel_models::refund::Refund;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for Refund {}
