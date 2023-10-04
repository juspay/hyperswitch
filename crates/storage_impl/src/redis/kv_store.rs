use std::{fmt::Debug, sync::Arc};

use common_utils::errors::CustomResult;
use redis_interface::errors::RedisError;
use router_derive::TryGetEnumVariant;
use serde::de;

use crate::{consts, KVRouterStore};

pub trait KvStorePartition {
    fn partition_number(key: PartitionKey<'_>, num_partitions: u8) -> u32 {
        crc32fast::hash(key.to_string().as_bytes()) % u32::from(num_partitions)
    }

    fn shard_key(key: PartitionKey<'_>, num_partitions: u8) -> String {
        format!("shard_{}", Self::partition_number(key, num_partitions))
    }
}

#[allow(unused)]
pub enum PartitionKey<'a> {
    MerchantIdPaymentId {
        merchant_id: &'a str,
        payment_id: &'a str,
    },
}

impl<'a> std::fmt::Display for PartitionKey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            PartitionKey::MerchantIdPaymentId {
                merchant_id,
                payment_id,
            } => f.write_str(&format!("mid_{merchant_id}_pid_{payment_id}")),
        }
    }
}

pub trait RedisConnInterface {
    fn get_redis_conn(
        &self,
    ) -> error_stack::Result<Arc<redis_interface::RedisConnectionPool>, RedisError>;
}

pub enum KvOperation<'a, S: serde::Serialize + Debug> {
    Set((&'a str, String)),
    SetNx(&'a str, S),
    Get(&'a str),
    Scan(&'a str),
}

#[derive(TryGetEnumVariant)]
#[error(RedisError(UnknownResult))]
pub enum KvResult<T: de::DeserializeOwned> {
    Get(T),
    Set(()),
    SetNx(redis_interface::HsetnxReply),
    Scan(Vec<T>),
}

pub async fn kv_wrapper<'a, T, D, S>(
    store: &KVRouterStore<D>,
    op: KvOperation<'a, S>,
    key: impl AsRef<str>,
) -> CustomResult<KvResult<T>, RedisError>
where
    T: de::DeserializeOwned,
    D: crate::database::store::DatabaseStore,
    S: serde::Serialize + Debug,
{
    let redis_conn = store.get_redis_conn()?;

    let key = key.as_ref();
    let type_name = std::any::type_name::<T>();

    match op {
        KvOperation::Set(value) => {
            redis_conn
                .set_hash_fields(key, value, Some(consts::KV_TTL))
                .await?;
            Ok(KvResult::Set(()))
        }
        KvOperation::Get(field) => {
            let result = redis_conn
                .get_hash_field_and_deserialize(key, field, type_name)
                .await?;
            Ok(KvResult::Get(result))
        }
        KvOperation::Scan(pattern) => {
            let result: Vec<T> = redis_conn.hscan_and_deserialize(key, pattern, None).await?;
            Ok(KvResult::Scan(result))
        }
        KvOperation::SetNx(field, value) => {
            let result = redis_conn
                .serialize_and_set_hash_field_if_not_exist(key, field, value, Some(consts::KV_TTL))
                .await?;
            Ok(KvResult::SetNx(result))
        }
    }
}
