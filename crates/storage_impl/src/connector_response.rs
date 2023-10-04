use diesel_models::connector_response::ConnectorResponse;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for ConnectorResponse {}
