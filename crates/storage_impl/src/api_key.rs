use common_utils::errors::CustomResult;
use diesel_models::api_keys as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::api_keys::ApiKeyInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> ApiKeyInterface for RouterStore<T> {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn insert_api_key(
        &self,
        api_key: storage::ApiKeyNew,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        api_key
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_api_key(
        &self,
        merchant_id: common_utils::id_type::MerchantId,
        key_id: common_utils::id_type::ApiKeyId,
        api_key: storage::ApiKeyUpdate,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let _merchant_id = merchant_id.clone();
        let _key_id = key_id.clone();
        let update_call = || async {
            storage::ApiKey::update_by_merchant_id_key_id(&conn, merchant_id, key_id, api_key)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            update_call().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            use error_stack::report;

            // We need to fetch api_key here because the key that's saved in cache in HashedApiKey.
            // Used function from storage model to reuse the connection that made here instead of
            // creating new.
            let api_key = storage::ApiKey::find_optional_by_merchant_id_key_id(
                &conn,
                &_merchant_id,
                &_key_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .ok_or(report!(errors::StorageError::ValueNotFound(format!(
                "ApiKey of {} not found",
                _key_id.get_string_repr()
            ))))?;

            cache::publish_and_redact(
                self,
                CacheKind::Accounts(api_key.hashed_api_key.into_inner().into()),
                update_call,
            )
            .await
        }
    }

    #[instrument(skip_all)]
    async fn revoke_api_key(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        key_id: &common_utils::id_type::ApiKeyId,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let delete_call = || async {
            storage::ApiKey::revoke_by_merchant_id_key_id(&conn, merchant_id, key_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        };
        #[cfg(not(feature = "accounts_cache"))]
        {
            delete_call().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            use error_stack::report;

            // We need to fetch api_key here because the key that's saved in cache in HashedApiKey.
            // Used function from storage model to reuse the connection that made here instead of
            // creating new.

            let api_key =
                storage::ApiKey::find_optional_by_merchant_id_key_id(&conn, merchant_id, key_id)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))?
                    .ok_or(report!(errors::StorageError::ValueNotFound(format!(
                        "ApiKey of {} not found",
                        key_id.get_string_repr()
                    ))))?;

            cache::publish_and_redact(
                self,
                CacheKind::Accounts(api_key.hashed_api_key.into_inner().into()),
                delete_call,
            )
            .await
        }
    }

    #[instrument(skip_all)]
    async fn find_api_key_by_merchant_id_key_id_optional(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        key_id: &common_utils::id_type::ApiKeyId,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::ApiKey::find_optional_by_merchant_id_key_id(&conn, merchant_id, key_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_api_key_by_hash_optional(
        &self,
        hashed_api_key: storage::HashedApiKey,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        let _hashed_api_key = hashed_api_key.clone();
        let find_call = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::ApiKey::find_optional_by_hashed_api_key(&conn, hashed_api_key)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            find_call().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            cache::get_or_populate_in_memory(
                self,
                &_hashed_api_key.into_inner(),
                find_call,
                &ACCOUNTS_CACHE,
            )
            .await
        }
    }

    #[instrument(skip_all)]
    async fn list_api_keys_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> CustomResult<Vec<storage::ApiKey>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::ApiKey::find_by_merchant_id(&conn, merchant_id, limit, offset)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}
