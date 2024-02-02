use time::ext::NumericalDuration;

use crate::{
    core::errors::{self, CustomResult},
    db::MockDb,
    types::storage::ephemeral_key::{EphemeralKey, EphemeralKeyNew},
};

#[async_trait::async_trait]
pub trait EphemeralKeyInterface {
    async fn create_ephemeral_key(
        &self,
        _ek: EphemeralKeyNew,
        _validity: i64,
    ) -> CustomResult<EphemeralKey, errors::StorageError>;
    async fn get_ephemeral_key(
        &self,
        _key: &str,
    ) -> CustomResult<EphemeralKey, errors::StorageError>;
    async fn delete_ephemeral_key(
        &self,
        _id: &str,
    ) -> CustomResult<EphemeralKey, errors::StorageError>;
}

mod storage {
    use common_utils::date_time;
    use error_stack::ResultExt;
    use redis_interface::HsetnxReply;
    use storage_impl::redis::kv_store::RedisConnInterface;
    use time::ext::NumericalDuration;

    use super::EphemeralKeyInterface;
    use crate::{
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::ephemeral_key::{EphemeralKey, EphemeralKeyNew},
    };

    #[async_trait::async_trait]
    impl EphemeralKeyInterface for Store {
                /// Asynchronously creates a new ephemeral key with the given EphemeralKeyNew and validity, and returns a CustomResult with the created EphemeralKey on success or a StorageError on failure. The method generates a secret key and an id key based on the new EphemeralKeyNew, calculates the expiration time, and then serializes and sets the key-value pairs in the Redis storage. If the keys already exist, it returns a StorageError for duplicate value. Otherwise, it sets the expiration for the keys in the Redis storage and returns the created EphemeralKey. If any error occurs during the process, it returns a StorageError with the appropriate context.
        async fn create_ephemeral_key(
            &self,
            new: EphemeralKeyNew,
            validity: i64,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            let secret_key = format!("epkey_{}", &new.secret);
            let id_key = format!("epkey_{}", &new.id);

            let created_at = date_time::now();
            let expires = created_at.saturating_add(validity.hours());
            let created_ek = EphemeralKey {
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
                    &[(&secret_key, &created_ek), (&id_key, &created_ek)],
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
                        .set_expire_at(&secret_key, expire_at)
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    self.get_redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .set_expire_at(&id_key, expire_at)
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    Ok(created_ek)
                }
                Err(er) => Err(er).change_context(errors::StorageError::KVError),
            }
        }
        /// Asynchronously retrieves an ephemeral key from the storage by its unique identifier.
        ///
        /// # Arguments
        ///
        /// * `key` - A string slice representing the unique identifier of the ephemeral key
        ///
        /// # Returns
        ///
        /// A `CustomResult` containing either the retrieved `EphemeralKey` or a `StorageError`
        ///
        /// # Errors
        ///
        /// An error of type `StorageError` is returned if the operation fails
        ///
        async fn get_ephemeral_key(
            &self,
            key: &str,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            let key = format!("epkey_{key}");
            self.get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .get_hash_field_and_deserialize(&key, "ephkey", "EphemeralKey")
                .await
                .change_context(errors::StorageError::KVError)
        }
                /// Asynchronously deletes an ephemeral key from the storage by its ID and secret, then returns the deleted key.
        async fn delete_ephemeral_key(
            &self,
            id: &str,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            let ek = self.get_ephemeral_key(id).await?;

            self.get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .delete_key(&format!("epkey_{}", &ek.id))
                .await
                .change_context(errors::StorageError::KVError)?;

            self.get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .delete_key(&format!("epkey_{}", &ek.secret))
                .await
                .change_context(errors::StorageError::KVError)?;
            Ok(ek)
        }
    }
}

#[async_trait::async_trait]
impl EphemeralKeyInterface for MockDb {
    /// Asynchronously creates a new ephemeral key with the given EphemeralKeyNew and validity, and returns a CustomResult containing the created EphemeralKey or a StorageError if an error occurs.
    async fn create_ephemeral_key(
            &self,
            ek: EphemeralKeyNew,
            validity: i64,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            let mut ephemeral_keys = self.ephemeral_keys.lock().await;
            let created_at = common_utils::date_time::now();
            let expires = created_at.saturating_add(validity.hours());
    
            let ephemeral_key = EphemeralKey {
                id: ek.id,
                merchant_id: ek.merchant_id,
                customer_id: ek.customer_id,
                created_at: created_at.assume_utc().unix_timestamp(),
                expires: expires.assume_utc().unix_timestamp(),
                secret: ek.secret,
            };
            ephemeral_keys.push(ephemeral_key.clone());
            Ok(ephemeral_key)
        }
    /// Asynchronously gets an ephemeral key from the storage based on the provided key.
    /// 
    /// # Arguments
    /// 
    /// * `key` - A reference to a string representing the key of the ephemeral key to retrieve.
    /// 
    /// # Returns
    /// 
    /// * If a matching ephemeral key is found, it returns a `CustomResult` containing the ephemeral key.
    /// * If no matching ephemeral key is found, it returns a `CustomResult` containing a `StorageError` indicating that the ephemeral key was not found.
    /// 
    async fn get_ephemeral_key(
        &self,
        key: &str,
    ) -> CustomResult<EphemeralKey, errors::StorageError> {
        match self
            .ephemeral_keys
            .lock()
            .await
            .iter()
            .find(|ephemeral_key| ephemeral_key.secret.eq(key))
        {
            Some(ephemeral_key) => Ok(ephemeral_key.clone()),
            None => Err(
                errors::StorageError::ValueNotFound("ephemeral key not found".to_string()).into(),
            ),
        }
    }
        /// Asynchronously deletes an ephemeral key from the storage by its ID.
        async fn delete_ephemeral_key(
            &self,
            id: &str,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            let mut ephemeral_keys = self.ephemeral_keys.lock().await;
            if let Some(pos) = ephemeral_keys.iter().position(|x| (*x.id).eq(id)) {
                let ek = ephemeral_keys.remove(pos);
                Ok(ek)
            } else {
                return Err(
                    errors::StorageError::ValueNotFound("ephemeral key not found".to_string()).into(),
                );
            }
        }
}
