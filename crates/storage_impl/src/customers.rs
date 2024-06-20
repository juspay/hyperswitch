use diesel_models::customers::Customer;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for Customer {}
