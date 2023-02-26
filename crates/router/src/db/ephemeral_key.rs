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
    use time::ext::NumericalDuration;

    use super::EphemeralKeyInterface;
    use crate::{
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::ephemeral_key::{EphemeralKey, EphemeralKeyNew},
    };

    #[async_trait::async_trait]
    impl EphemeralKeyInterface for Store {
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
                .redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .serialize_and_set_multiple_hash_field_if_not_exist(
                    &[(&secret_key, &created_ek), (&id_key, &created_ek)],
                    "ephkey",
                )
                .await
            {
                Ok(v) if v.contains(&HsetnxReply::KeyNotSet) => {
                    Err(errors::StorageError::DuplicateValue {
                        entity: "ephimeral key",
                        key: None,
                    }
                    .into())
                }
                Ok(_) => {
                    let expire_at = expires.assume_utc().unix_timestamp();
                    self.redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .set_expire_at(&secret_key, expire_at)
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    self.redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .set_expire_at(&id_key, expire_at)
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    Ok(created_ek)
                }
                Err(er) => Err(er).change_context(errors::StorageError::KVError),
            }
        }
        async fn get_ephemeral_key(
            &self,
            key: &str,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            let key = format!("epkey_{key}");
            self.redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .get_hash_field_and_deserialize(&key, "ephkey", "EphemeralKey")
                .await
                .change_context(errors::StorageError::KVError)
        }
        async fn delete_ephemeral_key(
            &self,
            id: &str,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            let ek = self.get_ephemeral_key(id).await?;

            self.redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .delete_key(&format!("epkey_{}", &ek.id))
                .await
                .change_context(errors::StorageError::KVError)?;

            self.redis_conn()
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
