use std::{fmt::Debug, sync::Arc};

use common_utils::errors::CustomResult;
use diesel_models::enums::MerchantStorageScheme;
use error_stack::report;
use redis_interface::errors::RedisError;
use router_derive::TryGetEnumVariant;
use router_env::logger;
use serde::de;

use crate::{kv_router_store::KVRouterStore, metrics, store::kv::TypedSql, UniqueConstraints};

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
        merchant_id: &'a common_utils::id_type::MerchantId,
        payment_id: &'a common_utils::id_type::PaymentId,
    },
    CombinationKey {
        combination: &'a str,
    },
    MerchantIdCustomerId {
        merchant_id: &'a common_utils::id_type::MerchantId,
        customer_id: &'a common_utils::id_type::CustomerId,
    },
    #[cfg(feature = "v2")]
    MerchantIdMerchantReferenceId {
        merchant_id: &'a common_utils::id_type::MerchantId,
        merchant_reference_id: &'a str,
    },
    MerchantIdPayoutId {
        merchant_id: &'a common_utils::id_type::MerchantId,
        payout_id: &'a common_utils::id_type::PayoutId,
    },
    MerchantIdPayoutAttemptId {
        merchant_id: &'a common_utils::id_type::MerchantId,
        payout_attempt_id: &'a str,
    },
    MerchantIdMandateId {
        merchant_id: &'a common_utils::id_type::MerchantId,
        mandate_id: &'a str,
    },
    #[cfg(feature = "v2")]
    GlobalId {
        id: &'a str,
    },
    #[cfg(feature = "v2")]
    GlobalPaymentId {
        id: &'a common_utils::id_type::GlobalPaymentId,
    },
}
// PartitionKey::MerchantIdPaymentId {merchant_id, payment_id}
impl std::fmt::Display for PartitionKey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            PartitionKey::MerchantIdPaymentId {
                merchant_id,
                payment_id,
            } => f.write_str(&format!(
                "mid_{}_pid_{}",
                merchant_id.get_string_repr(),
                payment_id.get_string_repr()
            )),
            PartitionKey::CombinationKey { combination } => f.write_str(combination),
            PartitionKey::MerchantIdCustomerId {
                merchant_id,
                customer_id,
            } => f.write_str(&format!(
                "mid_{}_cust_{}",
                merchant_id.get_string_repr(),
                customer_id.get_string_repr()
            )),
            #[cfg(feature = "v2")]
            PartitionKey::MerchantIdMerchantReferenceId {
                merchant_id,
                merchant_reference_id,
            } => f.write_str(&format!(
                "mid_{}_cust_{merchant_reference_id}",
                merchant_id.get_string_repr()
            )),
            PartitionKey::MerchantIdPayoutId {
                merchant_id,
                payout_id,
            } => f.write_str(&format!(
                "mid_{}_po_{}",
                merchant_id.get_string_repr(),
                payout_id.get_string_repr()
            )),
            PartitionKey::MerchantIdPayoutAttemptId {
                merchant_id,
                payout_attempt_id,
            } => f.write_str(&format!(
                "mid_{}_poa_{payout_attempt_id}",
                merchant_id.get_string_repr()
            )),
            PartitionKey::MerchantIdMandateId {
                merchant_id,
                mandate_id,
            } => f.write_str(&format!(
                "mid_{}_mandate_{mandate_id}",
                merchant_id.get_string_repr()
            )),

            #[cfg(feature = "v2")]
            PartitionKey::GlobalId { id } => f.write_str(&format!("global_cust_{id}")),
            #[cfg(feature = "v2")]
            PartitionKey::GlobalPaymentId { id } => {
                f.write_str(&format!("global_payment_{}", id.get_string_repr()))
            }
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

    let key = format!("{partition_key}");

    let type_name = std::any::type_name::<T>();
    let operation = op.to_string();

    let ttl = store.ttl_for_kv;

    let result = async {
        match op {
            KvOperation::Hset(value, sql) => {
                logger::debug!(kv_operation= %operation, value = ?value);

                redis_conn
                    .set_hash_fields(&key.into(), value, Some(ttl.into()))
                    .await?;

                store
                    .push_to_drainer_stream::<S>(sql, partition_key)
                    .await?;

                Ok(KvResult::Hset(()))
            }

            KvOperation::HGet(field) => {
                let result = redis_conn
                    .get_hash_field_and_deserialize(&key.into(), field, type_name)
                    .await?;
                Ok(KvResult::HGet(result))
            }

            KvOperation::Scan(pattern) => {
                let result: Vec<T> = redis_conn
                    .hscan_and_deserialize(&key.into(), pattern, None)
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
                    .serialize_and_set_hash_field_if_not_exist(&key.into(), field, value, Some(ttl))
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
                    .serialize_and_set_key_if_not_exist(&key.into(), value, Some(ttl.into()))
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
                let result = redis_conn
                    .get_and_deserialize_key(&key.into(), type_name)
                    .await?;
                Ok(KvResult::Get(result))
            }
        }
    };

    let attributes = router_env::metric_attributes!(("operation", operation.clone()));
    result
        .await
        .inspect(|_| {
            logger::debug!(kv_operation= %operation, status="success");
            metrics::KV_OPERATION_SUCCESSFUL.add(1, attributes);
        })
        .inspect_err(|err| {
            logger::error!(kv_operation = %operation, status="error", error = ?err);
            metrics::KV_OPERATION_FAILED.add(1, attributes);
        })
}

pub enum Op<'a> {
    Insert,
    Update(PartitionKey<'a>, &'a str, Option<&'a str>),
    Find,
}

impl std::fmt::Display for Op<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::Insert => f.write_str("insert"),
            Op::Find => f.write_str("find"),
            Op::Update(p_key, _, updated_by) => {
                f.write_str(&format!("update_{p_key} for updated_by_{updated_by:?}"))
            }
        }
    }
}

pub async fn decide_storage_scheme<T, D>(
    store: &KVRouterStore<T>,
    storage_scheme: MerchantStorageScheme,
    operation: Op<'_>,
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
                match Box::pin(kv_wrapper::<D, _, _>(
                    store,
                    KvOperation::<D>::HGet(field),
                    partition_key,
                ))
                .await
                {
                    Ok(_) => {
                        metrics::KV_SOFT_KILL_ACTIVE_UPDATE.add(1, &[]);
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
