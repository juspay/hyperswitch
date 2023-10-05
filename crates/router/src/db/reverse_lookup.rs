use error_stack::IntoReport;

use super::{cache, MockDb, Store};
use crate::{
    connection,
    errors::{self, CustomResult},
    types::storage::reverse_lookup::{ReverseLookup, ReverseLookupNew},
};

#[async_trait::async_trait]
pub trait ReverseLookupInterface {
    async fn insert_reverse_lookup(
        &self,
        _new: ReverseLookupNew,
    ) -> CustomResult<ReverseLookup, errors::StorageError>;
    async fn get_lookup_by_lookup_id(
        &self,
        _id: &str,
    ) -> CustomResult<ReverseLookup, errors::StorageError>;
}

#[async_trait::async_trait]
impl ReverseLookupInterface for Store {
    async fn insert_reverse_lookup(
        &self,
        new: ReverseLookupNew,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert(&conn).await.map_err(Into::into).into_report()
    }

    async fn get_lookup_by_lookup_id(
        &self,
        id: &str,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        let database_call = || async {
            let conn = connection::pg_connection_read(self).await?;
            ReverseLookup::find_by_lookup_id(id, &conn)
                .await
                .map_err(Into::into)
                .into_report()
        };
        cache::get_or_populate_redis(self, format!("reverse_lookup_{id}"), database_call).await
    }
}

#[async_trait::async_trait]
impl ReverseLookupInterface for MockDb {
    async fn insert_reverse_lookup(
        &self,
        new: ReverseLookupNew,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        let reverse_lookup_insert = ReverseLookup::from(new);
        self.reverse_lookups
            .lock()
            .await
            .push(reverse_lookup_insert.clone());
        Ok(reverse_lookup_insert)
    }
    async fn get_lookup_by_lookup_id(
        &self,
        lookup_id: &str,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        self.reverse_lookups
            .lock()
            .await
            .iter()
            .find(|reverse_lookup| reverse_lookup.lookup_id == lookup_id)
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No reverse lookup found for lookup_id = {}",
                    lookup_id
                ))
                .into(),
            )
            .cloned()
    }
}
