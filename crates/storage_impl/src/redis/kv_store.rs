use std::{fmt::Debug, sync::Arc};

use common_utils::errors::CustomResult;
use diesel_models::enums::MerchantStorageScheme;
use error_stack::report;
use redis_interface::errors::RedisError;
use router_derive::TryGetEnumVariant;
use router_env::logger;
use serde::de;

use crate::{metrics, store::kv::TypedSql, KVRouterStore, UniqueConstraints};

pub trait KvStorePartition {
    fn partition_number(key: PartitionKey<'_>, num_partitions: u8) -> u32 {
        crc32fast::hash(key.to_string().as_bytes()) % u32::from(num_partitions)
    }

    fn shard_key(key: PartitionKey<'_>, num_partitions: u8) -> String {
        format!("shard_{}", Self::partition_number(key, num_partitions))
    }
}

#[allow(unused)]
#[derive(Clone)]
pub enum PartitionKey<'a> {
    MerchantIdPaymentId {
        merchant_id: &'a str,
        payment_id: &'a str,
    },
    CombinationKey {
        combination: &'a str,
    },
    MerchantIdCustomerId {
        merchant_id: &'a str,
        customer_id: &'a str,
    },
    MerchantIdPayoutId {
        merchant_id: &'a str,
        payout_id: &'a str,
    },
    MerchantIdPayoutAttemptId {
        merchant_id: &'a str,
        payout_attempt_id: &'a str,
    },
    MerchantIdMandateId {
        merchant_id: &'a str,
        mandate_id: &'a str,
    },
}
// PartitionKey::MerchantIdPaymentId {merchant_id, payment_id}
impl<'a> std::fmt::Display for PartitionKey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            PartitionKey::MerchantIdPaymentId {
                merchant_id,
                payment_id,
            } => f.write_str(&format!("mid_{merchant_id}_pid_{payment_id}")),
            PartitionKey::CombinationKey { combination } => f.write_str(combination),
            PartitionKey::MerchantIdCustomerId {
                merchant_id,
                customer_id,
            } => f.write_str(&format!("mid_{merchant_id}_cust_{customer_id}")),
            PartitionKey::MerchantIdPayoutId {
                merchant_id,
                payout_id,
            } => f.write_str(&format!("mid_{merchant_id}_po_{payout_id}")),
            PartitionKey::MerchantIdPayoutAttemptId {
                merchant_id,
                payout_attempt_id,
            } => f.write_str(&format!("mid_{merchant_id}_poa_{payout_attempt_id}")),
            PartitionKey::MerchantIdMandateId {
                merchant_id,
                mandate_id,
            } => f.write_str(&format!("mid_{merchant_id}_mandate_{mandate_id}")),
        }
    }
}

pub trait RedisConnInterface {
    fn get_redis_conn(
        &self,
    ) -> error_stack::Result<Arc<redis_interface::RedisConnectionPool>, RedisError>;
}

/// An enum to represent what operation to do on
pub enum KvOperation<'a, S: serde::Serialize + Debug> {
    Hset((&'a str, String), TypedSql),
    SetNx(&'a S, TypedSql),
    HSetNx(&'a str, &'a S, TypedSql),
    HGet(&'a str),
    Get,
    Scan(&'a str),
}

#[derive(TryGetEnumVariant)]
#[error(RedisError::UnknownResult)]
pub enum KvResult<T: de::DeserializeOwned> {
    HGet(T),
    Get(T),
    Hset(()),
    SetNx(redis_interface::SetnxReply),
    HSetNx(redis_interface::HsetnxReply),
    Scan(Vec<T>),
}

impl<T> std::fmt::Display for KvOperation<'_, T>
where
    T: serde::Serialize + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KvOperation::Hset(_, _) => f.write_str("Hset"),
            KvOperation::SetNx(_, _) => f.write_str("Setnx"),
            KvOperation::HSetNx(_, _, _) => f.write_str("HSetNx"),
            KvOperation::HGet(_) => f.write_str("Hget"),
            KvOperation::Get => f.write_str("Get"),
            KvOperation::Scan(_) => f.write_str("Scan"),
        }
    }
}

pub async fn kv_wrapper<'a, T, D, S>(
    store: &KVRouterStore<D>,
    op: KvOperation<'a, S>,
    partition_key: PartitionKey<'a>,
) -> CustomResult<KvResult<T>, RedisError>
where
    T: de::DeserializeOwned,
    D: crate::database::store::DatabaseStore,
    S: serde::Serialize + Debug + KvStorePartition + UniqueConstraints + Sync,
{
    let redis_conn = store.get_redis_conn()?;

    let key = format!("{}", partition_key);

    let type_name = std::any::type_name::<T>();
    let operation = op.to_string();

    let ttl = store.ttl_for_kv;

    let result = async {
        match op {
            KvOperation::Hset(value, sql) => {
                logger::debug!(kv_operation= %operation, value = ?value);

                redis_conn
                    .set_hash_fields(&key, value, Some(ttl.into()))
                    .await?;

                store
                    .push_to_drainer_stream::<S>(sql, partition_key)
                    .await?;

                Ok(KvResult::Hset(()))
            }

            KvOperation::HGet(field) => {
                let result = redis_conn
                    .get_hash_field_and_deserialize(&key, field, type_name)
                    .await?;
                Ok(KvResult::HGet(result))
            }

            KvOperation::Scan(pattern) => {
                let result: Vec<T> = redis_conn
                    .hscan_and_deserialize(&key, pattern, None)
                    .await
                    .and_then(|result| {
                        if result.is_empty() {
                            Err(report!(RedisError::NotFound))
                        } else {
                            Ok(result)
                        }
                    })?;
                Ok(KvResult::Scan(result))
            }

            KvOperation::HSetNx(field, value, sql) => {
                logger::debug!(kv_operation= %operation, value = ?value);

                value.check_for_constraints(&redis_conn).await?;

                let result = redis_conn
                    .serialize_and_set_hash_field_if_not_exist(&key, field, value, Some(ttl))
                    .await?;

                if matches!(result, redis_interface::HsetnxReply::KeySet) {
                    store
                        .push_to_drainer_stream::<S>(sql, partition_key)
                        .await?;
                    Ok(KvResult::HSetNx(result))
                } else {
                    Err(report!(RedisError::SetNxFailed))
                }
            }

            KvOperation::SetNx(value, sql) => {
                logger::debug!(kv_operation= %operation, value = ?value);

                let result = redis_conn
                    .serialize_and_set_key_if_not_exist(&key, value, Some(ttl.into()))
                    .await?;

                value.check_for_constraints(&redis_conn).await?;

                if matches!(result, redis_interface::SetnxReply::KeySet) {
                    store
                        .push_to_drainer_stream::<S>(sql, partition_key)
                        .await?;
                    Ok(KvResult::SetNx(result))
                } else {
                    Err(report!(RedisError::SetNxFailed))
                }
            }

            KvOperation::Get => {
                let result = redis_conn.get_and_deserialize_key(&key, type_name).await?;
                Ok(KvResult::Get(result))
            }
        }
    };

    result
        .await
        .map(|result| {
            logger::debug!(kv_operation= %operation, status="success");
            let keyvalue = router_env::opentelemetry::KeyValue::new("operation", operation.clone());

            metrics::KV_OPERATION_SUCCESSFUL.add(&metrics::CONTEXT, 1, &[keyvalue]);
            result
        })
        .map_err(|err| {
            logger::error!(kv_operation = %operation, status="error", error = ?err);
            let keyvalue = router_env::opentelemetry::KeyValue::new("operation", operation);

            metrics::KV_OPERATION_FAILED.add(&metrics::CONTEXT, 1, &[keyvalue]);
            err
        })
}

pub enum Op<'a> {
    Insert,
    Update(PartitionKey<'a>, &'a str, Option<&'a str>),
    Find,
}

impl<'a> std::fmt::Display for Op<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::Insert => f.write_str("insert"),
            Op::Find => f.write_str("find"),
            Op::Update(p_key, _, updated_by) => {
                f.write_str(&format!("update_{} for updated_by_{:?}", p_key, updated_by))
            }
        }
    }
}

pub async fn decide_storage_scheme<'a, T, D>(
    store: &KVRouterStore<T>,
    storage_scheme: MerchantStorageScheme,
    operation: Op<'a>,
) -> MerchantStorageScheme
where
    D: de::DeserializeOwned
        + serde::Serialize
        + Debug
        + KvStorePartition
        + UniqueConstraints
        + Sync,
    T: crate::database::store::DatabaseStore,
{
    if store.soft_kill_mode {
        let ops = operation.to_string();
        let updated_scheme = match operation {
            Op::Insert => MerchantStorageScheme::PostgresOnly,
            Op::Find => MerchantStorageScheme::RedisKv,
            Op::Update(_, _, Some("postgres_only")) => MerchantStorageScheme::PostgresOnly,
            Op::Update(partition_key, field, Some(_updated_by)) => {
                match kv_wrapper::<D, _, _>(store, KvOperation::<D>::HGet(field), partition_key)
                    .await
                {
                    Ok(_) => {
                        metrics::KV_SOFT_KILL_ACTIVE_UPDATE.add(&metrics::CONTEXT, 1, &[]);
                        MerchantStorageScheme::RedisKv
                    }
                    Err(_) => MerchantStorageScheme::PostgresOnly,
                }
            }

            Op::Update(_, _, None) => MerchantStorageScheme::PostgresOnly,
        };

        let type_name = std::any::type_name::<D>();
        logger::info!(soft_kill_mode = "decide_storage_scheme", decided_scheme = %updated_scheme, configured_scheme = %storage_scheme,entity = %type_name, operation = %ops);

        updated_scheme
    } else {
        storage_scheme
    }
}
