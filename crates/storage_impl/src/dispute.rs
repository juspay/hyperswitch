use diesel_models::Dispute;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for Dispute {}
