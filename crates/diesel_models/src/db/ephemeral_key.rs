use common_utils::errors::{CustomResult};
use common_utils::pii::REDACTED;
use crate::services::{Store, MockDb};
use crate::cache::Cacheable;
use crate::db::cache::publish_and_redact;
use crate::errors::StorageError;
use crate::{ cache, CardInfo, enums, EphemeralKeyNew, EphemeralKey};
use crate::{domain::behaviour::Conversion, connection};
use crate::AddressNew;
use crate::address::AddressUpdateInternal;
use error_stack::{IntoReport, ResultExt};
use crate::{domain, errors};
use crate::domain::CustomerUpdate;

#[async_trait::async_trait]
pub trait EphemeralKeyInterface {
    async fn create_ephemeral_key(
        &self,
        _ek: EphemeralKeyNew,
        _validity: i64,
    ) -> CustomResult<EphemeralKey, StorageError>;
    async fn get_ephemeral_key(
        &self,
        _key: &str,
    ) -> CustomResult<EphemeralKey, StorageError>;
    async fn delete_ephemeral_key(
        &self,
        _id: &str,
    ) -> CustomResult<EphemeralKey, StorageError>;
}

mod storage {
    use common_utils::date_time;
    use error_stack::ResultExt;
    use redis_interface::HsetnxReply;
    use time::ext::NumericalDuration;

    use super::EphemeralKeyInterface;
    use common_utils::errors::{self, CustomResult};
    use crate::errors::StorageError;
    use crate::{
        services::Store,
        cache, CardInfo, enums, EphemeralKeyNew, EphemeralKey
    };

    #[async_trait::async_trait]
    impl EphemeralKeyInterface for Store {
        async fn create_ephemeral_key(
            &self,
            new: EphemeralKeyNew,
            validity: i64,
        ) -> CustomResult<EphemeralKey, StorageError> {
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
                .map_err(Into::<StorageError>::into)?
                .serialize_and_set_multiple_hash_field_if_not_exist(
                    &[(&secret_key, &created_ek), (&id_key, &created_ek)],
                    "ephkey",
                )
                .await
            {
                Ok(v) if v.contains(&HsetnxReply::KeyNotSet) => {
                    Err(StorageError::DuplicateValue {
                        entity: "ephimeral key",
                        key: None,
                    }
                    .into())
                }
                Ok(_) => {
                    let expire_at = expires.assume_utc().unix_timestamp();
                    self.redis_conn()
                        .map_err(Into::<StorageError>::into)?
                        .set_expire_at(&secret_key, expire_at)
                        .await
                        .change_context(StorageError::KVError)?;
                    self.redis_conn()
                        .map_err(Into::<StorageError>::into)?
                        .set_expire_at(&id_key, expire_at)
                        .await
                        .change_context(StorageError::KVError)?;
                    Ok(created_ek)
                }
                Err(er) => Err(er).change_context(StorageError::KVError),
            }
        }
        async fn get_ephemeral_key(
            &self,
            key: &str,
        ) -> CustomResult<EphemeralKey, StorageError> {
            let key = format!("epkey_{key}");
            self.redis_conn()
                .map_err(Into::<StorageError>::into)?
                .get_hash_field_and_deserialize(&key, "ephkey", "EphemeralKey")
                .await
                .change_context(StorageError::KVError)
        }
        async fn delete_ephemeral_key(
            &self,
            id: &str,
        ) -> CustomResult<EphemeralKey, StorageError> {
            let ek = self.get_ephemeral_key(id).await?;

            self.redis_conn()
                .map_err(Into::<StorageError>::into)?
                .delete_key(&format!("epkey_{}", &ek.id))
                .await
                .change_context(StorageError::KVError)?;

            self.redis_conn()
                .map_err(Into::<StorageError>::into)?
                .delete_key(&format!("epkey_{}", &ek.secret))
                .await
                .change_context(StorageError::KVError)?;
            Ok(ek)
        }
    }
}

#[async_trait::async_trait]
impl EphemeralKeyInterface for MockDb {
    async fn create_ephemeral_key(
        &self,
        ek: EphemeralKeyNew,
        validity: i64,
    ) -> CustomResult<EphemeralKey, StorageError> {
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
    async fn get_ephemeral_key(
        &self,
        key: &str,
    ) -> CustomResult<EphemeralKey, StorageError> {
        match self
            .ephemeral_keys
            .lock()
            .await
            .iter()
            .find(|ephemeral_key| ephemeral_key.secret.eq(key))
        {
            Some(ephemeral_key) => Ok(ephemeral_key.clone()),
            None => Err(
                StorageError::ValueNotFound("ephemeral key not found".to_string()).into(),
            ),
        }
    }
    async fn delete_ephemeral_key(
        &self,
        id: &str,
    ) -> CustomResult<EphemeralKey, StorageError> {
        let mut ephemeral_keys = self.ephemeral_keys.lock().await;
        if let Some(pos) = ephemeral_keys.iter().position(|x| (*x.id).eq(id)) {
            let ek = ephemeral_keys.remove(pos);
            Ok(ek)
        } else {
            return Err(
                StorageError::ValueNotFound("ephemeral key not found".to_string()).into(),
            );
        }
    }
}
