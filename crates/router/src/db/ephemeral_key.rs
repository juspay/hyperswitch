use crate::{
    core::errors::{self, CustomResult},
    types::storage::ephemeral_key::{EphemeralKey, EphemeralKeyNew},
};

#[async_trait::async_trait]
pub trait EphemeralKeyInterface {
    async fn create_ephemeral_key(
        &self,
        _ek: EphemeralKeyNew,
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

#[cfg(not(feature = "kv_store"))]
mod storage {
    use crate::{
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::ephemeral_key::{EphemeralKey, EphemeralKeyNew},
    };

    #[async_trait::async_trait]
    impl super::EphemeralKeyInterface for Store {
        async fn create_ephemeral_key(
            &self,
            _ek: EphemeralKeyNew,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            Err(errors::StorageError::KVError.into())
        }
        async fn get_ephemeral_key(
            &self,
            _key: &str,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            Err(errors::StorageError::KVError.into())
        }
        async fn delete_ephemeral_key(
            &self,
            _id: &str,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            Err(errors::StorageError::KVError.into())
        }
    }
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::date_time;
    use error_stack::{IntoReport, ResultExt};
    use fred::prelude::{KeysInterface, RedisValue};
    use time::ext::NumericalDuration;

    use crate::{
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::ephemeral_key::{EphemeralKey, EphemeralKeyNew},
    };

    #[async_trait::async_trait]
    impl super::EphemeralKeyInterface for Store {
        async fn create_ephemeral_key(
            &self,
            new: EphemeralKeyNew,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            let secret_key = new.secret.to_string();
            let id_key = new.id.to_string();

            let created_at = date_time::now();
            let expires = created_at.saturating_add(1.hours());
            let created_ek = EphemeralKey {
                id: new.id,
                created_at,
                expires,
                customer_id: new.customer_id,
                merchant_id: new.merchant_id,
                secret: new.secret,
            };
            let redis_value = &serde_json::to_string(&created_ek)
                .into_report()
                .change_context(errors::StorageError::KVError)?;

            let redis_map: Vec<(&str, RedisValue)> = vec![
                (&secret_key, redis_value.into()),
                (&id_key, redis_value.into()),
            ];
            match self
                .redis_conn
                .pool
                .msetnx::<u8, Vec<(&str, RedisValue)>>(redis_map)
                .await
            {
                Ok(1) => {
                    let expire_at = expires.assume_utc().unix_timestamp();
                    self.redis_conn
                        .set_expire_at(&secret_key, expire_at)
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    self.redis_conn
                        .set_expire_at(&id_key, expire_at)
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    Ok(created_ek)
                }
                Ok(0) => {
                    Err(errors::StorageError::DuplicateValue("ephimeral_key".to_string()).into())
                }
                Ok(i) => Err(errors::StorageError::KVError)
                    .into_report()
                    .attach_printable_lazy(|| format!("Invalid response for HSETNX: {}", i)),
                Err(er) => Err(er)
                    .into_report()
                    .change_context(errors::StorageError::KVError),
            }
        }
        async fn get_ephemeral_key(
            &self,
            key: &str,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            let value: String = self
                .redis_conn
                .get_key(key)
                .await
                .change_context(errors::StorageError::KVError)?;

            serde_json::from_str(&value)
                .into_report()
                .change_context(errors::StorageError::KVError)
        }
        async fn delete_ephemeral_key(
            &self,
            id: &str,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            let ek = self.get_ephemeral_key(id).await?;

            self.redis_conn
                .delete_key(&ek.id)
                .await
                .change_context(errors::StorageError::KVError)?;

            self.redis_conn
                .delete_key(&ek.secret)
                .await
                .change_context(errors::StorageError::KVError)?;
            Ok(ek)
        }
    }
}

#[async_trait::async_trait]
impl EphemeralKeyInterface for crate::db::MockDb {
    async fn create_ephemeral_key(
        &self,
        _ek: EphemeralKeyNew,
    ) -> CustomResult<EphemeralKey, errors::StorageError> {
        Err(errors::StorageError::KVError.into())
    }
    async fn get_ephemeral_key(
        &self,
        _key: &str,
    ) -> CustomResult<EphemeralKey, errors::StorageError> {
        Err(errors::StorageError::KVError.into())
    }
    async fn delete_ephemeral_key(
        &self,
        _id: &str,
    ) -> CustomResult<EphemeralKey, errors::StorageError> {
        Err(errors::StorageError::KVError.into())
    }
}
