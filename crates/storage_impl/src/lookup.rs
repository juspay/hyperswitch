use common_utils::errors::CustomResult;
use data_models::errors;
use diesel_models::{
    enums as storage_enums,
    reverse_lookup::{
        ReverseLookup as DieselReverseLookup, ReverseLookupNew as DieselReverseLookupNew,
    },
};
use error_stack::{IntoReport, ResultExt};
use redis_interface::SetnxReply;

use crate::{
    diesel_error_to_data_error,
    errors::RedisErrorExt,
    redis::kv_store::RedisConnInterface,
    utils::{self, try_redis_get_else_try_database_get},
    DatabaseStore, KVRouterStore, RouterStore,
};

#[async_trait::async_trait]
pub trait ReverseLookupInterface {
    async fn insert_reverse_lookup(
        &self,
        _new: DieselReverseLookupNew,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError>;
    async fn get_lookup_by_lookup_id(
        &self,
        _id: &str,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError>;
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ReverseLookupInterface for RouterStore<T> {
    async fn insert_reverse_lookup(
        &self,
        _new: DieselReverseLookupNew,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError> {
        Err(errors::StorageError::KVError).into_report()
    }

    async fn get_lookup_by_lookup_id(
        &self,
        _id: &str,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError> {
        Err(errors::StorageError::KVError).into_report()
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ReverseLookupInterface for KVRouterStore<T> {
    async fn insert_reverse_lookup(
        &self,
        new: DieselReverseLookupNew,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError> {
        let redis_conn = self
            .get_redis_conn()
            .map_err(|err| errors::StorageError::RedisError(err.to_string()))?;

        let created_rev_lookup = DieselReverseLookup {
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

    async fn get_lookup_by_lookup_id(
        &self,
        id: &str,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError> {
        let key = &format!("reverse_lookup_{}", id);

        let redis_conn = self
            .get_redis_conn()
            .map_err(|err| errors::StorageError::RedisError(err.to_string()))?;

        let database_call = || async {
            let conn = utils::pg_connection_read(self).await?;
            DieselReverseLookup::find_by_lookup_id(id, &conn)
                .await
                .map_err(|er| {
                    let new_err = diesel_error_to_data_error(er.current_context());
                    er.change_context(new_err)
                })
        };
        let redis_fut = redis_conn.get_and_deserialize_key(key, "ReverseLookup");

        Box::pin(try_redis_get_else_try_database_get(
            redis_fut,
            database_call,
        ))
        .await
    }
}
