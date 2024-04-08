use super::{MockDb, Store};
use crate::{
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
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<ReverseLookup, errors::StorageError>;
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use error_stack::report;
    use router_env::{instrument, tracing};

    use super::{ReverseLookupInterface, Store};
    use crate::{
        connection,
        errors::{self, CustomResult},
        types::storage::{
            enums,
            reverse_lookup::{ReverseLookup, ReverseLookupNew},
        },
    };

    #[async_trait::async_trait]
    impl ReverseLookupInterface for Store {
        #[instrument(skip_all)]
        async fn insert_reverse_lookup(
            &self,
            new: ReverseLookupNew,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<ReverseLookup, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            new.insert(&conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn get_lookup_by_lookup_id(
            &self,
            id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<ReverseLookup, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            ReverseLookup::find_by_lookup_id(id, &conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }
    }
}

#[cfg(feature = "kv_store")]
mod storage {
    use error_stack::{report, ResultExt};
    use redis_interface::SetnxReply;
    use router_env::{instrument, tracing};
    use storage_impl::redis::kv_store::{kv_wrapper, KvOperation, PartitionKey};

    use super::{ReverseLookupInterface, Store};
    use crate::{
        connection,
        core::errors::utils::RedisErrorExt,
        errors::{self, CustomResult},
        types::storage::{
            enums, kv,
            reverse_lookup::{ReverseLookup, ReverseLookupNew},
        },
        utils::db_utils,
    };

    #[async_trait::async_trait]
    impl ReverseLookupInterface for Store {
        #[instrument(skip_all)]
        async fn insert_reverse_lookup(
            &self,
            new: ReverseLookupNew,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<ReverseLookup, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    new.insert(&conn)
                        .await
                        .map_err(|error| report!(errors::StorageError::from(error)))
                }
                enums::MerchantStorageScheme::RedisKv => {
                    let created_rev_lookup = ReverseLookup {
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

                    match kv_wrapper::<ReverseLookup, _, _>(
                        self,
                        KvOperation::SetNx(&created_rev_lookup, redis_entry),
                        PartitionKey::CombinationKey{ combination : &format!("reverse_lookup_{}", &created_rev_lookup.lookup_id)},
                    )
                    .await
                    .map_err(|err| err.to_redis_failed_response(&created_rev_lookup.lookup_id))?
                    .try_into_setnx()
                    {
                        Ok(SetnxReply::KeySet) => Ok(created_rev_lookup),
                        Ok(SetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "reverse_lookup",
                            key: Some(created_rev_lookup.lookup_id.clone()),
                        }
                        .into()),
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }
        }

        #[instrument(skip_all)]
        async fn get_lookup_by_lookup_id(
            &self,
            id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<ReverseLookup, errors::StorageError> {
            let database_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                ReverseLookup::find_by_lookup_id(id, &conn)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
            };

            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let redis_fut = async {
                        kv_wrapper(
                            self,
                            KvOperation::<ReverseLookup>::Get,
                            PartitionKey::CombinationKey{ combination : &format!("reverse_lookup_{id}")},
                        )
                        .await?
                        .try_into_get()
                    };

                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        redis_fut,
                        database_call,
                    ))
                    .await
                }
            }
        }
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
        _storage_scheme: enums::MerchantStorageScheme,
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
