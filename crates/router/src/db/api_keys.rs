use error_stack::IntoReport;
#[cfg(feature = "accounts_cache")]
use storage_impl::redis::cache::CacheKind;
#[cfg(feature = "accounts_cache")]
use storage_impl::redis::cache::ACCOUNTS_CACHE;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait ApiKeyInterface {
    async fn insert_api_key(
        &self,
        api_key: storage::ApiKeyNew,
    ) -> CustomResult<storage::ApiKey, errors::StorageError>;

    async fn update_api_key(
        &self,
        merchant_id: String,
        key_id: String,
        api_key: storage::ApiKeyUpdate,
    ) -> CustomResult<storage::ApiKey, errors::StorageError>;

    async fn revoke_api_key(
        &self,
        merchant_id: &str,
        key_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn find_api_key_by_merchant_id_key_id_optional(
        &self,
        merchant_id: &str,
        key_id: &str,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError>;

    async fn find_api_key_by_hash_optional(
        &self,
        hashed_api_key: storage::HashedApiKey,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError>;

    async fn list_api_keys_by_merchant_id(
        &self,
        merchant_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> CustomResult<Vec<storage::ApiKey>, errors::StorageError>;
}

#[async_trait::async_trait]
impl ApiKeyInterface for Store {
        /// Asynchronously inserts a new API key into the storage.
    ///
    /// # Arguments
    /// * `api_key` - The new API key to be inserted into the storage.
    ///
    /// # Returns
    /// A `CustomResult` containing the inserted `storage::ApiKey` if successful, otherwise an `errors::StorageError`.
    ///
    /// # Errors
    /// If there is an error connecting to the database or inserting the API key, an `errors::StorageError` will be returned.
    ///
    async fn insert_api_key(
        &self,
        api_key: storage::ApiKeyNew,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        api_key
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }


        /// Asynchronously updates an API key for a given merchant. It first establishes a database connection, then performs the update operation on the API key. If the 'accounts_cache' feature is enabled, it retrieves the updated API key from the database, publishes it to the cache, and then returns the updated API key. If the 'accounts_cache' feature is not enabled, it simply performs the update operation on the API key and returns the result.
    async fn update_api_key(
        &self,
        merchant_id: String,
        key_id: String,
        api_key: storage::ApiKeyUpdate,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let _merchant_id = merchant_id.clone();
        let _key_id = key_id.clone();
        let update_call = || async {
            storage::ApiKey::update_by_merchant_id_key_id(&conn, merchant_id, key_id, api_key)
                .await
                .map_err(Into::into)
                .into_report()
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
            .map_err(Into::into)
            .into_report()?
            .ok_or(report!(errors::StorageError::ValueNotFound(format!(
                "ApiKey of {_key_id} not found"
            ))))?;

            super::cache::publish_and_redact(
                self,
                CacheKind::Accounts(api_key.hashed_api_key.into_inner().into()),
                update_call,
            )
            .await
        }
    }

        /// Asynchronously revokes an API key for the specified merchant. 
    /// Returns a CustomResult containing a boolean indicating whether the revocation was successful, or an errors::StorageError if an error occurred.
    async fn revoke_api_key(
        &self,
        merchant_id: &str,
        key_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let delete_call = || async {
            storage::ApiKey::revoke_by_merchant_id_key_id(&conn, merchant_id, key_id)
                .await
                .map_err(Into::into)
                .into_report()
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
                    .map_err(Into::into)
                    .into_report()?
                    .ok_or(report!(errors::StorageError::ValueNotFound(format!(
                        "ApiKey of {key_id} not found"
                    ))))?;

            super::cache::publish_and_redact(
                self,
                CacheKind::Accounts(api_key.hashed_api_key.into_inner().into()),
                delete_call,
            )
            .await
        }
    }

        /// Asynchronously finds an API key by the given merchant ID and key ID. If the API key is found, it is returned
    /// as Some(api_key), otherwise None is returned. Returns a CustomResult containing the found API key or a
    /// StorageError if an error occurs during the database operation.
    async fn find_api_key_by_merchant_id_key_id_optional(
        &self,
        merchant_id: &str,
        key_id: &str,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::ApiKey::find_optional_by_merchant_id_key_id(&conn, merchant_id, key_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously searches for an API key by its hashed value, returning an optional result.
    /// If the 'accounts_cache' feature is enabled, it may use an in-memory cache for faster retrieval.
    async fn find_api_key_by_hash_optional(
        &self,
        hashed_api_key: storage::HashedApiKey,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        let _hashed_api_key = hashed_api_key.clone();
        let find_call = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::ApiKey::find_optional_by_hashed_api_key(&conn, hashed_api_key)
                .await
                .map_err(Into::into)
                .into_report()
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            find_call().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::get_or_populate_in_memory(
                self,
                &_hashed_api_key.into_inner(),
                find_call,
                &ACCOUNTS_CACHE,
            )
            .await
        }
    }

        /// Retrieve a list of API keys associated with a specific merchant by their ID,
    /// with optional limit and offset parameters for pagination.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - The unique identifier of the merchant
    /// * `limit` - Optional parameter to limit the number of results returned
    /// * `offset` - Optional parameter to specify the offset for paginated results
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of `storage::ApiKey` if successful,
    /// otherwise an `errors::StorageError` is returned.
    ///
    async fn list_api_keys_by_merchant_id(
        &self,
        merchant_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> CustomResult<Vec<storage::ApiKey>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::ApiKey::find_by_merchant_id(&conn, merchant_id, limit, offset)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl ApiKeyInterface for MockDb {
        /// Asynchronously inserts a new API key into the storage, ensuring that the key_id is unique.
    ///
    /// # Arguments
    ///
    /// * `api_key` - The new API key to be inserted
    ///
    /// # Returns
    ///
    /// * `CustomResult<storage::ApiKey, errors::StorageError>` - Result containing the stored API key if successful, or a StorageError if the insertion fails
    ///
    async fn insert_api_key(
        &self,
        api_key: storage::ApiKeyNew,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        let mut locked_api_keys = self.api_keys.lock().await;
        // don't allow duplicate key_ids, a those would be a unique constraint violation in the
        // real db as it is used as the primary key
        if locked_api_keys.iter().any(|k| k.key_id == api_key.key_id) {
            Err(errors::StorageError::MockDbError)?;
        }
        let stored_key = storage::ApiKey {
            key_id: api_key.key_id,
            merchant_id: api_key.merchant_id,
            name: api_key.name,
            description: api_key.description,
            hashed_api_key: api_key.hashed_api_key,
            prefix: api_key.prefix,
            created_at: api_key.created_at,
            expires_at: api_key.expires_at,
            last_used: api_key.last_used,
        };
        locked_api_keys.push(stored_key.clone());

        Ok(stored_key)
    }

        /// Asynchronously updates the API key for the given merchant_id and key_id with the provided details. If the key is found, it is updated with the new values. If the key is not found, an error is returned. Returns the updated API key if successful.
    async fn update_api_key(
        &self,
        merchant_id: String,
        key_id: String,
        api_key: storage::ApiKeyUpdate,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        let mut locked_api_keys = self.api_keys.lock().await;
        // find a key with the given merchant_id and key_id and update, otherwise return an error
        let key_to_update = locked_api_keys
            .iter_mut()
            .find(|k| k.merchant_id == merchant_id && k.key_id == key_id)
            .ok_or(errors::StorageError::MockDbError)?;

        match api_key {
            storage::ApiKeyUpdate::Update {
                name,
                description,
                expires_at,
                last_used,
            } => {
                if let Some(name) = name {
                    key_to_update.name = name;
                }
                // only update these fields if the value was Some(_)
                if description.is_some() {
                    key_to_update.description = description;
                }
                if let Some(expires_at) = expires_at {
                    key_to_update.expires_at = expires_at;
                }
                if last_used.is_some() {
                    key_to_update.last_used = last_used
                }
            }
            storage::ApiKeyUpdate::LastUsedUpdate { last_used } => {
                key_to_update.last_used = Some(last_used);
            }
        }

        Ok(key_to_update.clone())
    }

        /// Asynchronously revokes an API key associated with a specific merchant.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - The ID of the merchant to revoke the API key for.
    /// * `key_id` - The ID of the API key to revoke.
    ///
    /// # Returns
    ///
    /// A `CustomResult` indicating whether the API key was successfully revoked or if an error occurred.
    ///
    /// # Errors
    ///
    /// An `errors::StorageError` is returned if an error occurs during the revocation process.
    ///
    async fn revoke_api_key(
        &self,
        merchant_id: &str,
        key_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let mut locked_api_keys = self.api_keys.lock().await;
        // find the key to remove, if it exists
        if let Some(pos) = locked_api_keys
            .iter()
            .position(|k| k.merchant_id == merchant_id && k.key_id == key_id)
        {
            // use `remove` instead of `swap_remove` so we have a consistent order, which might
            // matter to someone using limit/offset in `list_api_keys_by_merchant_id`
            locked_api_keys.remove(pos);
            Ok(true)
        } else {
            Ok(false)
        }
    }

        /// Asynchronously finds an API key by merchant ID and key ID, returning an optional result.
    async fn find_api_key_by_merchant_id_key_id_optional(
        &self,
        merchant_id: &str,
        key_id: &str,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        Ok(self
            .api_keys
            .lock()
            .await
            .iter()
            .find(|k| k.merchant_id == merchant_id && k.key_id == key_id)
            .cloned())
    }

        /// Asynchronously searches for an API key by its hashed value in the storage and returns an optional result.
    ///
    /// # Arguments
    ///
    /// * `hashed_api_key` - The hashed value of the API key to search for.
    ///
    /// # Returns
    ///
    /// An optional result containing the API key associated with the hashed value, or None if no matching key is found.
    ///
    /// # Errors
    ///
    /// Returns a StorageError if there is an issue with accessing the storage.
    ///
    async fn find_api_key_by_hash_optional(
        &self,
        hashed_api_key: storage::HashedApiKey,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        Ok(self
            .api_keys
            .lock()
            .await
            .iter()
            .find(|k| k.hashed_api_key == hashed_api_key)
            .cloned())
    }

        /// Retrieve a list of API keys associated with a specific merchant ID.
    ///
    /// This method takes a merchant ID, optional limit, and optional offset as parameters and returns
    /// a Result containing a vector of storage::ApiKey objects or an errors::StorageError if an error occurs.
    ///
    /// The method mimics the behavior of SQL limit and offset for pagination. It filters the API keys
    /// based on the provided merchant ID, applies the offset and limit constraints, and returns the
    /// filtered list of keys.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A reference to a string representing the merchant ID
    /// * `limit` - An optional limit for the number of keys to retrieve
    /// * `offset` - An optional offset for pagination
    ///
    /// # Returns
    ///
    /// A Result containing a vector of storage::ApiKey objects for the specified merchant ID, or an
    /// errors::StorageError if an error occurs.
    ///
    async fn list_api_keys_by_merchant_id(
        &self,
        merchant_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> CustomResult<Vec<storage::ApiKey>, errors::StorageError> {
        // mimic the SQL limit/offset behavior
        let offset: usize = if let Some(offset) = offset {
            if offset < 0 {
                Err(errors::StorageError::MockDbError)?;
            }
            offset
                .try_into()
                .map_err(|_| errors::StorageError::MockDbError)?
        } else {
            0
        };

        let limit: usize = if let Some(limit) = limit {
            if limit < 0 {
                Err(errors::StorageError::MockDbError)?;
            }
            limit
                .try_into()
                .map_err(|_| errors::StorageError::MockDbError)?
        } else {
            usize::MAX
        };

        let keys_for_merchant_id: Vec<storage::ApiKey> = self
            .api_keys
            .lock()
            .await
            .iter()
            .filter(|k| k.merchant_id == merchant_id)
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();

        Ok(keys_for_merchant_id)
    }
}

#[cfg(test)]
mod tests {
    use storage_impl::redis::{
        cache::{CacheKind, ACCOUNTS_CACHE},
        kv_store::RedisConnInterface,
        pub_sub::PubSubInterface,
    };
    use time::macros::datetime;

    use crate::{
        db::{api_keys::ApiKeyInterface, cache, MockDb},
        types::storage,
    };

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
        /// Asynchronously tests the functionality of the MockDb API key interface by performing various operations such as inserting, finding, updating, listing, and revoking API keys. The method creates a MockDb instance, inserts two API keys, finds a specific API key by merchant ID and key ID, updates the last used timestamp of a specific API key, lists all API keys by merchant ID, and revokes a specific API key by merchant ID and key ID. Assertions are used to verify the expected results of each operation.
    async fn test_mockdb_api_key_interface() {
        #[allow(clippy::expect_used)]
        let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
            .await
            .expect("Failed to create Mock store");

        let key1 = mockdb
            .insert_api_key(storage::ApiKeyNew {
                key_id: "key_id1".into(),
                merchant_id: "merchant1".into(),
                name: "Key 1".into(),
                description: None,
                hashed_api_key: "hashed_key1".to_string().into(),
                prefix: "abc".into(),
                created_at: datetime!(2023-02-01 0:00),
                expires_at: Some(datetime!(2023-03-01 0:00)),
                last_used: None,
            })
            .await
            .unwrap();

        mockdb
            .insert_api_key(storage::ApiKeyNew {
                key_id: "key_id2".into(),
                merchant_id: "merchant1".into(),
                name: "Key 2".into(),
                description: None,
                hashed_api_key: "hashed_key2".to_string().into(),
                prefix: "abc".into(),
                created_at: datetime!(2023-03-01 0:00),
                expires_at: None,
                last_used: None,
            })
            .await
            .unwrap();

        let found_key1 = mockdb
            .find_api_key_by_merchant_id_key_id_optional("merchant1", "key_id1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found_key1.key_id, key1.key_id);
        assert!(mockdb
            .find_api_key_by_merchant_id_key_id_optional("merchant1", "does_not_exist")
            .await
            .unwrap()
            .is_none());

        mockdb
            .update_api_key(
                "merchant1".into(),
                "key_id1".into(),
                storage::ApiKeyUpdate::LastUsedUpdate {
                    last_used: datetime!(2023-02-04 1:11),
                },
            )
            .await
            .unwrap();
        let updated_key1 = mockdb
            .find_api_key_by_merchant_id_key_id_optional("merchant1", "key_id1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated_key1.last_used, Some(datetime!(2023-02-04 1:11)));

        assert_eq!(
            mockdb
                .list_api_keys_by_merchant_id("merchant1", None, None)
                .await
                .unwrap()
                .len(),
            2
        );
        mockdb.revoke_api_key("merchant1", "key_id1").await.unwrap();
        assert_eq!(
            mockdb
                .list_api_keys_by_merchant_id("merchant1", None, None)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
        /// Asynchronously tests the API keys cache by performing the following steps:
    /// 1. Creates a new mock database with default Redis settings and subscribes to "hyperswitch_invalidate" channel
    /// 2. Inserts a new API key into the database and retrieves the hashed API key
    /// 3. Retrieves the API key from the cache or populates it in memory if not found
    /// 4. Publishes and redacts the API key in the cache
    /// 5. Asserts that the API key is no longer present in the cache
    async fn test_api_keys_cache() {
        #[allow(clippy::expect_used)]
        let db = MockDb::new(&redis_interface::RedisSettings::default())
            .await
            .expect("Failed to create Mock store");

        let redis_conn = db.get_redis_conn().unwrap();
        redis_conn
            .subscribe("hyperswitch_invalidate")
            .await
            .unwrap();

        let merchant_id = "test_merchant";
        let api = storage::ApiKeyNew {
            key_id: "test_key".into(),
            merchant_id: merchant_id.into(),
            name: "My test key".into(),
            description: None,
            hashed_api_key: "a_hashed_key".to_string().into(),
            prefix: "pre".into(),
            created_at: datetime!(2023-06-01 0:00),
            expires_at: None,
            last_used: None,
        };

        let api = db.insert_api_key(api).await.unwrap();

        let hashed_api_key = api.hashed_api_key.clone();
        let find_call = || async {
            db.find_api_key_by_hash_optional(hashed_api_key.clone())
                .await
        };
        let _: Option<storage::ApiKey> = cache::get_or_populate_in_memory(
            &db,
            &format!("{}_{}", merchant_id, hashed_api_key.clone().into_inner()),
            find_call,
            &ACCOUNTS_CACHE,
        )
        .await
        .unwrap();

        let delete_call = || async { db.revoke_api_key(merchant_id, &api.key_id).await };

        cache::publish_and_redact(
            &db,
            CacheKind::Accounts(
                format!("{}_{}", merchant_id, hashed_api_key.clone().into_inner()).into(),
            ),
            delete_call,
        )
        .await
        .unwrap();

        assert!(
            ACCOUNTS_CACHE
                .get_val::<storage::ApiKey>(&format!(
                    "{}_{}",
                    merchant_id,
                    hashed_api_key.into_inner()
                ),)
                .await
                .is_none()
        )
    }
}
