use diesel_models::capture::Capture;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for Capture {}
