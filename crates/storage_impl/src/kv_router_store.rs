use std::{fmt::Debug, sync::Arc};

use common_enums::enums::MerchantStorageScheme;
use common_utils::{fallback_reverse_lookup_not_found, types::keymanager::KeyManagerState};
use diesel_models::{errors::DatabaseError, kv};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_key_store::MerchantKeyStore,
};
#[cfg(not(feature = "payouts"))]
use hyperswitch_domain_models::{PayoutAttemptInterface, PayoutsInterface};
use masking::StrongSecret;
use redis_interface::{errors::RedisError, types::HsetnxReply, RedisConnectionPool};
use router_env::logger;
use serde::de;

#[cfg(not(feature = "payouts"))]
pub use crate::database::store::Store;
pub use crate::{database::store::DatabaseStore, mock_db::MockDb};
use crate::{
    database::store::PgPool,
    diesel_error_to_data_error,
    errors::{self, RedisErrorExt, StorageResult},
    lookup::ReverseLookupInterface,
    metrics,
    redis::kv_store::{
        decide_storage_scheme, kv_wrapper, KvOperation, KvStorePartition, Op, PartitionKey,
        RedisConnInterface,
    },
    utils::{find_all_combined_kv_database, try_redis_get_else_try_database_get},
    RouterStore, TenantConfig, UniqueConstraints,
};

#[derive(Debug, Clone)]
pub struct KVRouterStore<T: DatabaseStore> {
    pub router_store: RouterStore<T>,
    pub key_manager_state: Option<KeyManagerState>,
    drainer_stream_name: String,
    drainer_num_partitions: u8,
    pub ttl_for_kv: u32,
    pub request_id: Option<String>,
    pub soft_kill_mode: bool,
}

impl<T: DatabaseStore> KVRouterStore<T> {
    pub fn get_keymanager_state(&self) -> Result<&KeyManagerState, errors::StorageError> {
        self.key_manager_state
            .as_ref()
            .ok_or_else(|| errors::StorageError::DecryptionError)
    }
}

pub struct InsertResourceParams<'a> {
    pub insertable: kv::Insertable,
    pub reverse_lookups: Vec<String>,
    pub key: PartitionKey<'a>,
    // secondary key
    pub identifier: String,
    // type of resource Eg: "payment_attempt"
    pub resource_type: &'static str,
}

pub struct UpdateResourceParams<'a> {
    pub updateable: kv::Updateable,
    pub operation: Op<'a>,
}

pub struct FilterResourceParams<'a> {
    pub key: PartitionKey<'a>,
    pub pattern: &'static str,
    pub limit: Option<i64>,
}

pub enum FindResourceBy<'a> {
    Id(String, PartitionKey<'a>),
    LookupId(String),
}

pub trait DomainType: Debug + Sync + Conversion {}
impl<T: Debug + Sync + Conversion> DomainType for T {}

/// Storage model with all required capabilities for KV operations
pub trait StorageModel<D: Conversion>:
    de::DeserializeOwned
    + serde::Serialize
    + Debug
    + KvStorePartition
    + UniqueConstraints
    + Sync
    + Send
    + ReverseConversion<D>
{
}

impl<T, D> StorageModel<D> for T
where
    T: de::DeserializeOwned
        + serde::Serialize
        + Debug
        + KvStorePartition
        + UniqueConstraints
        + Sync
        + Send
        + ReverseConversion<D>,
    D: DomainType,
{
}

#[async_trait::async_trait]
impl<T> DatabaseStore for KVRouterStore<T>
where
    RouterStore<T>: DatabaseStore,
    T: DatabaseStore,
{
    type Config = (RouterStore<T>, String, u8, u32, Option<bool>);
    async fn new(
        config: Self::Config,
        tenant_config: &dyn TenantConfig,
        _test_transaction: bool,
        key_manager_state: Option<KeyManagerState>,
    ) -> StorageResult<Self> {
        let (router_store, _, drainer_num_partitions, ttl_for_kv, soft_kill_mode) = config;
        let drainer_stream_name = format!("{}_{}", tenant_config.get_schema(), config.1);
        Ok(Self::from_store(
            router_store,
            drainer_stream_name,
            drainer_num_partitions,
            ttl_for_kv,
            soft_kill_mode,
            key_manager_state,
        ))
    }
    fn get_master_pool(&self) -> &PgPool {
        self.router_store.get_master_pool()
    }
    fn get_replica_pool(&self) -> &PgPool {
        self.router_store.get_replica_pool()
    }

    fn get_accounts_master_pool(&self) -> &PgPool {
        self.router_store.get_accounts_master_pool()
    }

    fn get_accounts_replica_pool(&self) -> &PgPool {
        self.router_store.get_accounts_replica_pool()
    }
}

impl<T: DatabaseStore> RedisConnInterface for KVRouterStore<T> {
    fn get_redis_conn(&self) -> error_stack::Result<Arc<RedisConnectionPool>, RedisError> {
        self.router_store.get_redis_conn()
    }
}

impl<T: DatabaseStore> KVRouterStore<T> {
    pub fn from_store(
        store: RouterStore<T>,
        drainer_stream_name: String,
        drainer_num_partitions: u8,
        ttl_for_kv: u32,
        soft_kill: Option<bool>,
        key_manager_state: Option<KeyManagerState>,
    ) -> Self {
        let request_id = store.request_id.clone();

        Self {
            router_store: store,
            drainer_stream_name,
            drainer_num_partitions,
            ttl_for_kv,
            request_id,
            soft_kill_mode: soft_kill.unwrap_or(false),
            key_manager_state,
        }
    }

    pub fn master_key(&self) -> &StrongSecret<Vec<u8>> {
        self.router_store.master_key()
    }

    pub fn get_drainer_stream_name(&self, shard_key: &str) -> String {
        format!("{{{}}}_{}", shard_key, self.drainer_stream_name)
    }

    pub async fn push_to_drainer_stream<R>(
        &self,
        redis_entry: kv::TypedSql,
        partition_key: PartitionKey<'_>,
    ) -> error_stack::Result<(), RedisError>
    where
        R: KvStorePartition,
    {
        let global_id = format!("{partition_key}");
        let request_id = self.request_id.clone().unwrap_or_default();

        let shard_key = R::shard_key(partition_key, self.drainer_num_partitions);
        let stream_name = self.get_drainer_stream_name(&shard_key);
        self.router_store
            .cache_store
            .redis_conn
            .stream_append_entry(
                &stream_name.into(),
                &redis_interface::RedisEntryId::AutoGeneratedID,
                redis_entry
                    .to_field_value_pairs(request_id, global_id)
                    .change_context(RedisError::JsonSerializationFailed)?,
            )
            .await
            .map(|_| metrics::KV_PUSHED_TO_DRAINER.add(1, &[]))
            .inspect_err(|error| {
                metrics::KV_FAILED_TO_PUSH_TO_DRAINER.add(1, &[]);
                logger::error!(?error, "Failed to add entry in drainer stream");
            })
            .change_context(RedisError::StreamAppendFailed)
    }

    pub async fn find_resource_by_id<D, R, M>(
        &self,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
        find_resource_db_fut: R,
        find_by: FindResourceBy<'_>,
    ) -> error_stack::Result<D, errors::StorageError>
    where
        D: DomainType,
        M: StorageModel<D>,
        R: futures::Future<Output = error_stack::Result<M, DatabaseError>> + Send,
    {
        let database_call = || async {
            find_resource_db_fut.await.map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
        };
        let storage_scheme = Box::pin(decide_storage_scheme::<T, M>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        let res = || async {
            match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let (field, key) = match find_by {
                        FindResourceBy::Id(field, key) => (field, key),
                        FindResourceBy::LookupId(lookup_id) => {
                            let lookup = fallback_reverse_lookup_not_found!(
                                self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                                    .await,
                                database_call().await
                            );
                            (
                                lookup.clone().sk_id,
                                PartitionKey::CombinationKey {
                                    combination: &lookup.clone().pk_id,
                                },
                            )
                        }
                    };

                    Box::pin(try_redis_get_else_try_database_get(
                        async {
                            Box::pin(kv_wrapper(self, KvOperation::<M>::HGet(&field), key))
                                .await?
                                .try_into_hget()
                        },
                        database_call,
                    ))
                    .await
                }
            }
        };
        res()
            .await?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    pub async fn find_optional_resource_by_id<D, R, M>(
        &self,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
        find_resource_db_fut: R,
        find_by: FindResourceBy<'_>,
    ) -> error_stack::Result<Option<D>, errors::StorageError>
    where
        D: DomainType,
        M: StorageModel<D>,
        R: futures::Future<Output = error_stack::Result<Option<M>, DatabaseError>> + Send,
    {
        let database_call = || async {
            find_resource_db_fut.await.map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
        };
        let storage_scheme = Box::pin(decide_storage_scheme::<T, M>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        let res = || async {
            match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let (field, key) = match find_by {
                        FindResourceBy::Id(field, key) => (field, key),
                        FindResourceBy::LookupId(lookup_id) => {
                            let lookup = fallback_reverse_lookup_not_found!(
                                self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                                    .await,
                                database_call().await
                            );
                            (
                                lookup.clone().sk_id,
                                PartitionKey::CombinationKey {
                                    combination: &lookup.clone().pk_id,
                                },
                            )
                        }
                    };

                    Box::pin(try_redis_get_else_try_database_get(
                        async {
                            Box::pin(kv_wrapper(self, KvOperation::<M>::HGet(&field), key))
                                .await?
                                .try_into_hget()
                                .map(Some)
                        },
                        database_call,
                    ))
                    .await
                }
            }
        };
        match res().await? {
            Some(resource) => Ok(Some(
                resource
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)?,
            )),
            None => Ok(None),
        }
    }

    pub async fn insert_resource<D, R, M>(
        &self,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
        create_resource_fut: R,
        resource_new: M,
        InsertResourceParams {
            insertable,
            reverse_lookups,
            key,
            identifier,
            resource_type,
        }: InsertResourceParams<'_>,
    ) -> error_stack::Result<D, errors::StorageError>
    where
        D: Debug + Sync + Conversion,
        M: StorageModel<D>,
        R: futures::Future<Output = error_stack::Result<M, DatabaseError>> + Send,
    {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, M>(
            self,
            storage_scheme,
            Op::Insert,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => create_resource_fut.await.map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            }),
            MerchantStorageScheme::RedisKv => {
                let key_str = key.to_string();
                let reverse_lookup_entry = |v: String| diesel_models::ReverseLookupNew {
                    sk_id: identifier.clone(),
                    pk_id: key_str.clone(),
                    lookup_id: v,
                    source: resource_type.to_string(),
                    updated_by: storage_scheme.to_string(),
                };
                let results = reverse_lookups
                    .into_iter()
                    .map(|v| self.insert_reverse_lookup(reverse_lookup_entry(v), storage_scheme));

                futures::future::try_join_all(results).await?;

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Insert {
                        insertable: Box::new(insertable),
                    },
                };
                match Box::pin(kv_wrapper::<M, _, _>(
                    self,
                    KvOperation::<M>::HSetNx(&identifier, &resource_new, redis_entry),
                    key.clone(),
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key.to_string()))?
                .try_into_hsetnx()
                {
                    Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                        entity: resource_type,
                        key: Some(key_str),
                    }
                    .into()),
                    Ok(HsetnxReply::KeySet) => Ok(resource_new),
                    Err(er) => Err(er).change_context(errors::StorageError::KVError),
                }
            }
        }?
        .convert(
            self.get_keymanager_state()
                .attach_printable("Missing KeyManagerState")?,
            key_store.key.get_inner(),
            key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(errors::StorageError::DecryptionError)
    }

    pub async fn update_resource<D, R, M>(
        &self,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
        update_resource_fut: R,
        updated_resource: M,
        UpdateResourceParams {
            updateable,
            operation,
        }: UpdateResourceParams<'_>,
    ) -> error_stack::Result<D, errors::StorageError>
    where
        D: Debug + Sync + Conversion,
        M: StorageModel<D>,
        R: futures::Future<Output = error_stack::Result<M, DatabaseError>> + Send,
    {
        match operation {
            Op::Update(key, field, updated_by) => {
                let storage_scheme = Box::pin(decide_storage_scheme::<_, M>(
                    self,
                    storage_scheme,
                    Op::Update(key.clone(), field, updated_by),
                ))
                .await;
                match storage_scheme {
                    MerchantStorageScheme::PostgresOnly => {
                        update_resource_fut.await.map_err(|error| {
                            let new_err = diesel_error_to_data_error(*error.current_context());
                            error.change_context(new_err)
                        })
                    }
                    MerchantStorageScheme::RedisKv => {
                        let key_str = key.to_string();
                        let redis_value = serde_json::to_string(&updated_resource)
                            .change_context(errors::StorageError::SerializationFailed)?;

                        let redis_entry = kv::TypedSql {
                            op: kv::DBOperation::Update {
                                updatable: Box::new(updateable),
                            },
                        };
                        Box::pin(kv_wrapper::<(), _, _>(
                            self,
                            KvOperation::<M>::Hset((field, redis_value), redis_entry),
                            key,
                        ))
                        .await
                        .map_err(|err| err.to_redis_failed_response(&key_str))?
                        .try_into_hset()
                        .change_context(errors::StorageError::KVError)?;
                        Ok(updated_resource)
                    }
                }
            }
            _ => Err(errors::StorageError::KVError.into()),
        }?
        .convert(
            self.get_keymanager_state()
                .attach_printable("Missing KeyManagerState")?,
            key_store.key.get_inner(),
            key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(errors::StorageError::DecryptionError)
    }
    pub async fn filter_resources<D, R, M>(
        &self,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
        filter_resource_db_fut: R,
        filter_fn: impl Fn(&M) -> bool,
        FilterResourceParams {
            key,
            pattern,
            limit,
        }: FilterResourceParams<'_>,
    ) -> error_stack::Result<Vec<D>, errors::StorageError>
    where
        D: Debug + Sync + Conversion,
        M: StorageModel<D>,
        R: futures::Future<Output = error_stack::Result<Vec<M>, DatabaseError>> + Send,
    {
        let db_call = || async {
            filter_resource_db_fut.await.map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
        };
        let resources = match storage_scheme {
            MerchantStorageScheme::PostgresOnly => db_call().await,
            MerchantStorageScheme::RedisKv => {
                let redis_fut = async {
                    let kv_result = Box::pin(kv_wrapper::<M, _, _>(
                        self,
                        KvOperation::<M>::Scan(pattern),
                        key,
                    ))
                    .await?
                    .try_into_scan();
                    kv_result.map(|records| records.into_iter().filter(filter_fn).collect())
                };

                Box::pin(find_all_combined_kv_database(redis_fut, db_call, limit)).await
            }
        }?;
        let resource_futures = resources
            .into_iter()
            .map(|pm| async {
                pm.convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
            })
            .collect::<Vec<_>>();
        futures::future::try_join_all(resource_futures).await
    }
}

#[cfg(not(feature = "payouts"))]
impl<T: DatabaseStore> PayoutAttemptInterface for KVRouterStore<T> {}
#[cfg(not(feature = "payouts"))]
impl<T: DatabaseStore> PayoutsInterface for KVRouterStore<T> {}
