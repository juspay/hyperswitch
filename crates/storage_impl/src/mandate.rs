use diesel_models::Mandate;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for Mandate {}
