use diesel_models::address::Address;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for Address {}
