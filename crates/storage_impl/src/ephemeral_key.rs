// mod storage {
    use common_utils::date_time;

    #[cfg(feature = "v2")]
    use common_utils::id_type;
    
    #[cfg(feature = "v2")]
    use masking::PeekInterface;
    
    #[cfg(feature = "v2")]
    use redis_interface::errors::RedisError;
    
    use redis_interface::HsetnxReply;
    use time::ext::NumericalDuration;

    // #[cfg(feature = "v2")]
    // use crate::types::storage::ephemeral_key::{
    //     storage::EphemeralKeyType, storage::EphemeralKeyTypeNew, storage::ResourceType,
    // };

    use error_stack::ResultExt;

use common_utils::errors::CustomResult;
use diesel_models::ephemeral_key as storage;

use router_env::{instrument, tracing};

use sample::ephemeral_key::EphemeralKeyInterface;

use crate::{errors, DatabaseStore, RouterStore, redis::kv_store::RedisConnInterface};

#[async_trait::async_trait]
impl<T: DatabaseStore> EphemeralKeyInterface for RouterStore<T> {
    type Error = errors::StorageError;

        #[cfg(feature = "v1")]
        #[instrument(skip_all)]
        async fn create_ephemeral_key(
            &self,
            new: storage::EphemeralKeyNew,
            validity: i64,
        ) -> CustomResult<storage::EphemeralKey, errors::StorageError> {
            let secret_key = format!("epkey_{}", &new.secret);
            let id_key = format!("epkey_{}", &new.id);

            let created_at = date_time::now();
            let expires = created_at.saturating_add(validity.hours());
            let created_ek = storage::EphemeralKey {
                id: new.id,
                created_at: created_at.assume_utc().unix_timestamp(),
                expires: expires.assume_utc().unix_timestamp(),
                customer_id: new.customer_id,
                merchant_id: new.merchant_id,
                secret: new.secret,
            };

            match self
                .get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .serialize_and_set_multiple_hash_field_if_not_exist(
                    &[
                        (&secret_key.as_str().into(), &created_ek),
                        (&id_key.as_str().into(), &created_ek),
                    ],
                    "ephkey",
                    None,
                )
                .await
            {
                Ok(v) if v.contains(&HsetnxReply::KeyNotSet) => {
                    Err(errors::StorageError::DuplicateValue {
                        entity: "ephemeral key",
                        key: None,
                    }
                    .into())
                }
                Ok(_) => {
                    let expire_at = expires.assume_utc().unix_timestamp();
                    self.get_redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .set_expire_at(&secret_key.into(), expire_at)
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    self.get_redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .set_expire_at(&id_key.into(), expire_at)
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    Ok(created_ek)
                }
                Err(er) => Err(er).change_context(errors::StorageError::KVError),
            }
        }

        #[cfg(feature = "v2")]
        #[instrument(skip_all)]
        async fn create_ephemeral_key(
            &self,
            new: storage::EphemeralKeyTypeNew,
            validity: i64,
        ) -> CustomResult<storage::EphemeralKeyType, errors::StorageError> {
            let created_at = date_time::now();
            let expires = created_at.saturating_add(validity.hours());
            let id_key = new.id.generate_redis_key();

            let created_ephemeral_key = storage::EphemeralKeyType {
                id: new.id,
                created_at,
                expires,
                customer_id: new.customer_id,
                merchant_id: new.merchant_id,
                secret: new.secret,
                resource_type: new.resource_type,
            };
            let secret_key = created_ephemeral_key.generate_secret_key();

            match self
                .get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .serialize_and_set_multiple_hash_field_if_not_exist(
                    &[
                        (&secret_key.as_str().into(), &created_ephemeral_key),
                        (&id_key.as_str().into(), &created_ephemeral_key),
                    ],
                    "ephkey",
                    None,
                )
                .await
            {
                Ok(v) if v.contains(&HsetnxReply::KeyNotSet) => {
                    Err(errors::StorageError::DuplicateValue {
                        entity: "ephemeral key",
                        key: None,
                    }
                    .into())
                }
                Ok(_) => {
                    let expire_at = expires.assume_utc().unix_timestamp();
                    self.get_redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .set_expire_at(&secret_key.into(), expire_at)
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    self.get_redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .set_expire_at(&id_key.into(), expire_at)
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    Ok(created_ephemeral_key)
                }
                Err(er) => Err(er).change_context(errors::StorageError::KVError),
            }
        }

        #[cfg(feature = "v1")]
        #[instrument(skip_all)]
        async fn get_ephemeral_key(
            &self,
            key: &str,
        ) -> CustomResult<storage::EphemeralKey, errors::StorageError> {
            let key = format!("epkey_{key}");
            self.get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .get_hash_field_and_deserialize(&key.into(), "ephkey", "storage::EphemeralKey")
                .await
                .change_context(errors::StorageError::KVError)
        }

        #[cfg(feature = "v2")]
        #[instrument(skip_all)]
        async fn get_ephemeral_key(
            &self,
            key: &str,
        ) -> CustomResult<storage::EphemeralKeyType, errors::StorageError> {
            let key = format!("epkey_{key}");
            self.get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .get_hash_field_and_deserialize(&key.into(), "ephkey", "storage::EphemeralKeyType")
                .await
                .change_context(errors::StorageError::KVError)
        }

        #[cfg(feature = "v1")]
        async fn delete_ephemeral_key(
            &self,
            id: &str,
        ) -> CustomResult<storage::EphemeralKey, errors::StorageError> {
            let ek = self.get_ephemeral_key(id).await?;

            self.get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .delete_key(&format!("epkey_{}", &ek.id).into())
                .await
                .change_context(errors::StorageError::KVError)?;

            self.get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .delete_key(&format!("epkey_{}", &ek.secret).into())
                .await
                .change_context(errors::StorageError::KVError)?;
            Ok(ek)
        }

        #[cfg(feature = "v2")]
        async fn delete_ephemeral_key(
            &self,
            id: &str,
        ) -> CustomResult<storage::EphemeralKeyType, errors::StorageError> {
            let ephemeral_key = self.get_ephemeral_key(id).await?;
            let redis_id_key = ephemeral_key.id.generate_redis_key();
            let secret_key = ephemeral_key.generate_secret_key();

            self.get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .delete_key(&redis_id_key.as_str().into())
                .await
                .map_err(|err| match err.current_context() {
                    RedisError::NotFound => {
                        err.change_context(errors::StorageError::ValueNotFound(redis_id_key))
                    }
                    _ => err.change_context(errors::StorageError::KVError),
                })?;

            self.get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .delete_key(&secret_key.as_str().into())
                .await
                .map_err(|err| match err.current_context() {
                    RedisError::NotFound => {
                        err.change_context(errors::StorageError::ValueNotFound(secret_key))
                    }
                    _ => err.change_context(errors::StorageError::KVError),
                })?;
            Ok(ephemeral_key)
        }
    }
// }