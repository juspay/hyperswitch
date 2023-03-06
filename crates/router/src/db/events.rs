use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait EventInterface {
    async fn insert_event(
        &self,
        event: storage::EventNew,
    ) -> CustomResult<storage::Event, errors::StorageError>;
}

#[async_trait::async_trait]
impl EventInterface for Store {
    async fn insert_event(
        &self,
        event: storage::EventNew,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        event.insert(&conn).await.map_err(Into::into).into_report()
    }
}

#[async_trait::async_trait]
impl EventInterface for MockDb {
    async fn insert_event(
        &self,
        _event: storage::EventNew,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
