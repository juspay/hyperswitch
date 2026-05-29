use diesel_models::authentication::Authentication;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for Authentication {}
