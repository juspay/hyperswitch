use diesel_models::reverse_lookup::ReverseLookup;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for ReverseLookup {}
