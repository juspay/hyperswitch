use error_stack::{IntoReport, ResultExt};
use redis_interface::SetnxReply;
use router_env::{instrument, tracing};
use storage_impl::redis::kv_store::RedisConnInterface;

use super::{MockDb, Store};
use crate::{
    core::errors::utils::RedisErrorExt,
    errors::{self, CustomResult},
    types::storage::{
        enums,
        reverse_lookup::{ReverseLookup, ReverseLookupNew},
    },
};

#[async_trait::async_trait]
pub trait ReverseLookupInterface {
    async fn insert_reverse_lookup(
        &self,
        _new: ReverseLookupNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<ReverseLookup, errors::StorageError>;
    async fn get_lookup_by_lookup_id(
        &self,
        _id: &str,
    ) -> CustomResult<ReverseLookup, errors::StorageError>;
}

#[async_trait::async_trait]
impl ReverseLookupInterface for Store {
    #[instrument(skip_all)]
    async fn insert_reverse_lookup(
        &self,
        new: ReverseLookupNew,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        let redis_conn = self
            .get_redis_conn()
            .map_err(errors::StorageError::RedisError)?;

        let created_rev_lookup = ReverseLookup {
            lookup_id: new.lookup_id.clone(),
            sk_id: new.sk_id.clone(),
            pk_id: new.pk_id.clone(),
            source: new.source.clone(),
            updated_by: storage_scheme.to_string(),
        };

        let ttl = self.ttl_for_kv.saturating_add(self.reverse_lookup_offset);

        match redis_conn
            .serialize_and_set_key_if_not_exist(
                &format!("reverse_lookup_{}", &created_rev_lookup.lookup_id),
                &created_rev_lookup,
                Some(ttl.into()),
            )
            .await
            .map_err(|err| err.to_redis_failed_response(&created_rev_lookup.lookup_id))
        {
            Ok(SetnxReply::KeySet) => Ok(created_rev_lookup),
            Ok(SetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                entity: "reverse_lookup",
                key: Some(created_rev_lookup.lookup_id.clone()),
            })
            .into_report(),
            Err(er) => Err(er).change_context(errors::StorageError::KVError),
        }
    }

    #[instrument(skip_all)]
    async fn get_lookup_by_lookup_id(
        &self,
        id: &str,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        let key = &format!("reverse_lookup_{}", id);

        let redis_conn = self
            .get_redis_conn()
            .map_err(errors::StorageError::RedisError)?;

        redis_conn
            .get_and_deserialize_key(key, "ReverseLookup")
            .await
            .map_err(errors::StorageError::RedisError)
            .into_report()
    }
}

#[async_trait::async_trait]
impl ReverseLookupInterface for MockDb {
    async fn insert_reverse_lookup(
        &self,
        new: ReverseLookupNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        let reverse_lookup_insert = ReverseLookup::from(new);
        self.reverse_lookups
            .lock()
            .await
            .push(reverse_lookup_insert.clone());
        Ok(reverse_lookup_insert)
    }

    async fn get_lookup_by_lookup_id(
        &self,
        lookup_id: &str,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        self.reverse_lookups
            .lock()
            .await
            .iter()
            .find(|reverse_lookup| reverse_lookup.lookup_id == lookup_id)
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No reverse lookup found for lookup_id = {}",
                    lookup_id
                ))
                .into(),
            )
            .cloned()
    }
}
