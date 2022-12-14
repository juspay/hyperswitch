use storage_models::{errors, CustomResult};

use crate::{
    connection::pg_connection,
    types::storage::reverse_lookup::{ReverseLookup, ReverseLookupNew},
};

#[async_trait::async_trait]
pub trait ReverseLookupInterface {
    async fn insert_reverse_lookup(
        &self,
        _new: ReverseLookupNew,
    ) -> CustomResult<ReverseLookup, errors::DatabaseError>;
    async fn get_lookup_by_lookup_id(
        &self,
        _id: &str,
    ) -> CustomResult<ReverseLookup, errors::DatabaseError>;
}

#[async_trait::async_trait]
impl ReverseLookupInterface for super::Store {
    async fn insert_reverse_lookup(
        &self,
        new: ReverseLookupNew,
    ) -> CustomResult<ReverseLookup, errors::DatabaseError> {
        let conn = pg_connection(&self.master_pool).await;
        new.insert(&conn).await
    }

    async fn get_lookup_by_lookup_id(
        &self,
        id: &str,
    ) -> CustomResult<ReverseLookup, errors::DatabaseError> {
        let conn = pg_connection(&self.master_pool).await;
        ReverseLookup::find_by_lookup_id(id, &conn).await
    }
}

#[async_trait::async_trait]
impl ReverseLookupInterface for super::MockDb {
    async fn insert_reverse_lookup(
        &self,
        _new: ReverseLookupNew,
    ) -> CustomResult<ReverseLookup, errors::DatabaseError> {
        Err(errors::DatabaseError::NotFound.into())
    }
    async fn get_lookup_by_lookup_id(
        &self,
        _id: &str,
    ) -> CustomResult<ReverseLookup, errors::DatabaseError> {
        Err(errors::DatabaseError::NotFound.into())
    }
}
