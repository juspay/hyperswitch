use diesel_models::PaymentMethod;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for PaymentMethod {}