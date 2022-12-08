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
    use common_utils::{date_time, ext_traits::StringExt};
    use error_stack::{IntoReport, ResultExt};
    use redis_interface::RedisValue;
    use time::ext::NumericalDuration;

    use super::EphemeralKeyInterface;
    use crate::{
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::ephemeral_key::{EphemeralKey, EphemeralKeyNew},
        utils,
    };

    #[async_trait::async_trait]
    impl EphemeralKeyInterface for Store {
        async fn create_ephemeral_key(
            &self,
            new: EphemeralKeyNew,
            validity: i64,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            let secret_key = new.secret.to_string();
            let id_key = new.id.to_string();

            let created_at = date_time::now();
            let expires = created_at.saturating_add(validity.hours());
            let created_ek = EphemeralKey {
                id: new.id,
                created_at,
                expires,
                customer_id: new.customer_id,
                merchant_id: new.merchant_id,
                secret: new.secret,
            };
            let redis_value = &utils::Encode::<EphemeralKey>::encode_to_string_of_json(&created_ek)
                .change_context(errors::StorageError::KVError)
                .attach_printable("Unable to serialize ephemeral key")?;

            let redis_map: Vec<(&str, RedisValue)> = vec![
                (&secret_key, redis_value.into()),
                (&id_key, redis_value.into()),
            ];
            match self
                .redis_conn
                .msetnx::<Vec<(&str, RedisValue)>>(redis_map)
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
                    .attach_printable_lazy(|| format!("Invalid response for HSETNX: {i}")),
                Err(er) => Err(er).change_context(errors::StorageError::KVError),
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

            value
                .parse_struct("EphemeralKey")
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
impl EphemeralKeyInterface for MockDb {
    async fn create_ephemeral_key(
        &self,
        _ek: EphemeralKeyNew,
        _validity: i64,
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
