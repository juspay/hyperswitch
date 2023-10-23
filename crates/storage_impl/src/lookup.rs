use common_utils::errors::CustomResult;
use data_models::errors;
use diesel_models::{
    enums as storage_enums, kv,
    reverse_lookup::{
        ReverseLookup as DieselReverseLookup, ReverseLookupNew as DieselReverseLookupNew,
    },
};
use error_stack::{IntoReport, ResultExt};
use redis_interface::SetnxReply;

use crate::{
    diesel_error_to_data_error,
    redis::kv_store::{kv_wrapper, KvOperation},
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
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError>;
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ReverseLookupInterface for RouterStore<T> {
    async fn insert_reverse_lookup(
        &self,
        new: DieselReverseLookupNew,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError> {
        let conn = self
            .get_master_pool()
            .get()
            .await
            .into_report()
            .change_context(errors::StorageError::DatabaseConnectionError)?;
        new.insert(&conn).await.map_err(|er| {
            let new_err = diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
    }

    async fn get_lookup_by_lookup_id(
        &self,
        id: &str,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError> {
        let conn = utils::pg_connection_read(self).await?;
        DieselReverseLookup::find_by_lookup_id(id, &conn)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ReverseLookupInterface for KVRouterStore<T> {
    async fn insert_reverse_lookup(
        &self,
        new: DieselReverseLookupNew,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError> {
        match storage_scheme {
            storage_enums::MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .insert_reverse_lookup(new, storage_scheme)
                    .await
            }
            storage_enums::MerchantStorageScheme::RedisKv => {
                let created_rev_lookup = DieselReverseLookup {
                    lookup_id: new.lookup_id.clone(),
                    sk_id: new.sk_id.clone(),
                    pk_id: new.pk_id.clone(),
                    source: new.source.clone(),
                    updated_by: storage_scheme.to_string(),
                };
                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Insert {
                        insertable: kv::Insertable::ReverseLookUp(new),
                    },
                };

                match kv_wrapper::<DieselReverseLookup, _, _>(
                    self,
                    KvOperation::SetNx(&created_rev_lookup, redis_entry),
                    format!("reverse_lookup_{}", &created_rev_lookup.lookup_id),
                )
                .await
                .change_context(errors::StorageError::KVError)?
                .try_into_setnx()
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
        }
    }

    async fn get_lookup_by_lookup_id(
        &self,
        id: &str,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<DieselReverseLookup, errors::StorageError> {
        let database_call = || async {
            self.router_store
                .get_lookup_by_lookup_id(id, storage_scheme)
                .await
        };
        match storage_scheme {
            storage_enums::MerchantStorageScheme::PostgresOnly => database_call().await,
            storage_enums::MerchantStorageScheme::RedisKv => {
                let redis_fut = async {
                    kv_wrapper(
                        self,
                        KvOperation::<DieselReverseLookup>::Get,
                        format!("reverse_lookup_{id}"),
                    )
                    .await?
                    .try_into_get()
                };

                try_redis_get_else_try_database_get(redis_fut, database_call).await
            }
        }
    }
}
