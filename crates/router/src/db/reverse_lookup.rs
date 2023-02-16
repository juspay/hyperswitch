use error_stack::ResultExt;
use storage_models::{errors, StorageResult};

use super::{MockDb, Store};
use crate::{
    connection::pg_connection,
    types::storage::reverse_lookup::{ReverseLookup, ReverseLookupNew},
};

#[async_trait::async_trait]
pub trait ReverseLookupInterface {
    async fn insert_reverse_lookup(&self, _new: ReverseLookupNew) -> StorageResult<ReverseLookup>;
    async fn get_lookup_by_lookup_id(&self, _id: &str) -> StorageResult<ReverseLookup>;
}

#[async_trait::async_trait]
impl ReverseLookupInterface for Store {
    async fn insert_reverse_lookup(&self, new: ReverseLookupNew) -> StorageResult<ReverseLookup> {
        let conn = pg_connection(&self.master_pool)
            .await
            .change_context(errors::DatabaseError::DatabaseConnectionError)?;
        new.insert(&conn).await
    }

    async fn get_lookup_by_lookup_id(&self, id: &str) -> StorageResult<ReverseLookup> {
        let conn = pg_connection(&self.master_pool)
            .await
            .change_context(errors::DatabaseError::DatabaseConnectionError)?;
        ReverseLookup::find_by_lookup_id(id, &conn).await
    }
}

#[async_trait::async_trait]
impl ReverseLookupInterface for MockDb {
    async fn insert_reverse_lookup(&self, _new: ReverseLookupNew) -> StorageResult<ReverseLookup> {
        Err(errors::DatabaseError::NotFound.into())
    }
    async fn get_lookup_by_lookup_id(&self, _id: &str) -> StorageResult<ReverseLookup> {
        Err(errors::DatabaseError::NotFound.into())
    }
}
