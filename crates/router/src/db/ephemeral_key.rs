#[cfg(feature = "v2")]
use common_utils::id_type;
use time::ext::NumericalDuration;

#[cfg(feature = "v2")]
use crate::types::storage::ephemeral_key::{ClientSecretType, ClientSecretTypeNew};
use crate::{
    core::errors::{self, CustomResult},
    db::MockDb,
    types::storage::ephemeral_key::{EphemeralKey, EphemeralKeyNew},
};

#[async_trait::async_trait]
pub trait EphemeralKeyInterface {
    #[cfg(feature = "v1")]
    async fn create_ephemeral_key(
        &self,
        _ek: EphemeralKeyNew,
        _validity: i64,
    ) -> CustomResult<EphemeralKey, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn get_ephemeral_key(
        &self,
        _key: &str,
    ) -> CustomResult<EphemeralKey, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn delete_ephemeral_key(
        &self,
        _id: &str,
    ) -> CustomResult<EphemeralKey, errors::StorageError>;
}

#[async_trait::async_trait]
pub trait ClientSecretInterface {
    #[cfg(feature = "v2")]
    async fn create_client_secret(
        &self,
        _ek: ClientSecretTypeNew,
        _validity: i64,
    ) -> CustomResult<ClientSecretType, errors::StorageError>;

    #[cfg(feature = "v2")]
    async fn get_client_secret(
        &self,
        _key: &str,
    ) -> CustomResult<ClientSecretType, errors::StorageError>;

    #[cfg(feature = "v2")]
    async fn delete_client_secret(
        &self,
        _id: &str,
    ) -> CustomResult<ClientSecretType, errors::StorageError>;
}

mod storage {
    use common_utils::date_time;
    #[cfg(feature = "v2")]
    use common_utils::id_type;
    use error_stack::ResultExt;
    #[cfg(feature = "v2")]
    use masking::PeekInterface;
    #[cfg(feature = "v2")]
    use redis_interface::errors::RedisError;
    use redis_interface::HsetnxReply;
    use router_env::{instrument, tracing};
    use storage_impl::redis::kv_store::RedisConnInterface;
    use time::ext::NumericalDuration;

    use super::{ClientSecretInterface, EphemeralKeyInterface};
    #[cfg(feature = "v2")]
    use crate::types::storage::ephemeral_key::{ClientSecretType, ClientSecretTypeNew};
    use crate::{
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::ephemeral_key::{EphemeralKey, EphemeralKeyNew},
    };

    #[async_trait::async_trait]
    impl EphemeralKeyInterface for Store {
        #[cfg(feature = "v1")]
        #[instrument(skip_all)]
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

        #[cfg(feature = "v1")]
        #[instrument(skip_all)]
        async fn get_ephemeral_key(
            &self,
            key: &str,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
            let key = format!("epkey_{key}");
            self.get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .get_hash_field_and_deserialize(&key.into(), "ephkey", "EphemeralKey")
                .await
                .change_context(errors::StorageError::KVError)
        }

        #[cfg(feature = "v1")]
        async fn delete_ephemeral_key(
            &self,
            id: &str,
        ) -> CustomResult<EphemeralKey, errors::StorageError> {
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
    }

    #[async_trait::async_trait]
    impl ClientSecretInterface for Store {
        #[cfg(feature = "v2")]
        #[instrument(skip_all)]
        async fn create_client_secret(
            &self,
            new: ClientSecretTypeNew,
            validity: i64,
        ) -> CustomResult<ClientSecretType, errors::StorageError> {
            let created_at = date_time::now();
            let expires = created_at.saturating_add(validity.hours());
            let id_key = new.id.generate_redis_key();

            let created_client_secret = ClientSecretType {
                id: new.id,
                created_at,
                expires,
                merchant_id: new.merchant_id,
                secret: new.secret,
                resource_id: new.resource_id,
            };
            let secret_key = created_client_secret.generate_secret_key();

            match self
                .get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .serialize_and_set_multiple_hash_field_if_not_exist(
                    &[
                        (&secret_key.as_str().into(), &created_client_secret),
                        (&id_key.as_str().into(), &created_client_secret),
                    ],
                    "csh",
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
                    Ok(created_client_secret)
                }
                Err(er) => Err(er).change_context(errors::StorageError::KVError),
            }
        }

        #[cfg(feature = "v2")]
        #[instrument(skip_all)]
        async fn get_client_secret(
            &self,
            key: &str,
        ) -> CustomResult<ClientSecretType, errors::StorageError> {
            let key = format!("cs_{key}");
            self.get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)?
                .get_hash_field_and_deserialize(&key.into(), "csh", "ClientSecretType")
                .await
                .change_context(errors::StorageError::KVError)
        }

        #[cfg(feature = "v2")]
        async fn delete_client_secret(
            &self,
            id: &str,
        ) -> CustomResult<ClientSecretType, errors::StorageError> {
            let client_secret = self.get_client_secret(id).await?;
            let redis_id_key = client_secret.id.generate_redis_key();
            let secret_key = client_secret.generate_secret_key();

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
            Ok(client_secret)
        }
    }
}

#[async_trait::async_trait]
impl EphemeralKeyInterface for MockDb {
    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v1")]
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

#[async_trait::async_trait]
impl ClientSecretInterface for MockDb {
    #[cfg(feature = "v2")]
    async fn create_client_secret(
        &self,
        new: ClientSecretTypeNew,
        validity: i64,
    ) -> CustomResult<ClientSecretType, errors::StorageError> {
        todo!()
    }

    #[cfg(feature = "v2")]
    async fn get_client_secret(
        &self,
        key: &str,
    ) -> CustomResult<ClientSecretType, errors::StorageError> {
        todo!()
    }

    #[cfg(feature = "v2")]
    async fn delete_client_secret(
        &self,
        id: &str,
    ) -> CustomResult<ClientSecretType, errors::StorageError> {
        todo!()
    }
}
