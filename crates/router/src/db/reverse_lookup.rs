use error_stack::IntoReport;

use super::{cache, MockDb, Store};
use crate::{
    connection::pg_connection,
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
        let conn = pg_connection(&self.master_pool).await?;
        new.insert(&conn).await.map_err(Into::into).into_report()
    }

    async fn get_lookup_by_lookup_id(
        &self,
        id: &str,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        let database_call = || async {
            let conn = pg_connection(&self.master_pool).await?;
            ReverseLookup::find_by_lookup_id(id, &conn)
                .await
                .map_err(Into::into)
                .into_report()
        };
        cache::get_or_populate_cache(self, id, database_call).await
    }
}

#[async_trait::async_trait]
impl ReverseLookupInterface for MockDb {
    async fn insert_reverse_lookup(
        &self,
        _new: ReverseLookupNew,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        Err(errors::StorageError::MockDbError.into())
    }
    async fn get_lookup_by_lookup_id(
        &self,
        _id: &str,
    ) -> CustomResult<ReverseLookup, errors::StorageError> {
        Err(errors::StorageError::MockDbError.into())
    }
}
