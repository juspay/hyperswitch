use error_stack::IntoReport;

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

    async fn update_api_key(
        &self,
        merchant_id: String,
        key_id: String,
        api_key: storage::ApiKeyUpdate,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::ApiKey::update_by_merchant_id_key_id(&conn, merchant_id, key_id, api_key)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn revoke_api_key(
        &self,
        merchant_id: &str,
        key_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::ApiKey::revoke_by_merchant_id_key_id(&conn, merchant_id, key_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

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

    async fn find_api_key_by_hash_optional(
        &self,
        hashed_api_key: storage::HashedApiKey,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::ApiKey::find_optional_by_hashed_api_key(&conn, hashed_api_key)
            .await
            .map_err(Into::into)
            .into_report()
    }

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

    async fn update_api_key(
        &self,
        merchant_id: String,
        key_id: String,
        api_key: storage::ApiKeyUpdate,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        let mut locked_api_keys = self.api_keys.lock().await;
        // find a key with the given merchant_id and key_id and update, otherwise return an error
        let mut key_to_update = locked_api_keys
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

    async fn list_api_keys_by_merchant_id(
        &self,
        merchant_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> CustomResult<Vec<storage::ApiKey>, errors::StorageError> {
        // first gather all the keys for the given merchant_id, then apply any limit or offset, if
        // requested. This uses memory inefficiently if a limit/offset is requested, but should be
        // sufficient for the mockdb usecase
        let mut keys_for_merchant_id: Vec<storage::ApiKey> = self
            .api_keys
            .lock()
            .await
            .iter()
            .filter(|k| k.merchant_id == merchant_id)
            .cloned()
            .collect();

        // mimic the SQL limit/offset behavior
        if let Some(offset) = offset {
            if offset < 0 {
                Err(errors::StorageError::MockDbError)?;
            }
            let offset: usize = offset
                .try_into()
                .map_err(|_| errors::StorageError::MockDbError)?;
            keys_for_merchant_id = keys_for_merchant_id.into_iter().skip(offset).collect();
        };

        if let Some(limit) = limit {
            if limit < 0 {
                Err(errors::StorageError::MockDbError)?;
            }
            let limit: usize = limit
                .try_into()
                .map_err(|_| errors::StorageError::MockDbError)?;
            keys_for_merchant_id.truncate(limit);
        }

        Ok(keys_for_merchant_id)
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use crate::{
        db::{api_keys::ApiKeyInterface, MockDb},
        types::storage,
    };

    // ignored for now because we need to do some finagling with redis to get it to run, since the
    // mockdb needs to connect to a real redis instance
    #[tokio::test]
    async fn test_mockdb_api_key_interface() {
        let mockdb = MockDb::new(&Default::default()).await;

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
}
