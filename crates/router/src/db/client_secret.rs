#[cfg(feature = "v2")]
use common_utils::{date_time, id_type};
#[cfg(feature = "v2")]
use error_stack::ResultExt;
#[cfg(feature = "v2")]
use hyperswitch_masking::PeekInterface;
#[cfg(feature = "v2")]
use redis_interface::{errors::RedisError, HsetnxReply};
#[cfg(feature = "v2")]
use router_env::{instrument, tracing};
#[cfg(feature = "v2")]
use storage_impl::redis::kv_store::RedisConnInterface;
#[cfg(feature = "v2")]
use time::ext::NumericalDuration;

#[cfg(feature = "v2")]
use crate::core::errors::{self, CustomResult};
#[cfg(feature = "v2")]
use crate::types::storage::client_secret::{ClientSecretType, ClientSecretTypeNew};
use crate::{db::MockDb, services::Store};

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
                    entity: "client secret",
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
