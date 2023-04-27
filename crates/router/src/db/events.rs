use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait EventInterface {
    async fn insert_event(
        &self,
        event: storage::EventNew,
    ) -> CustomResult<storage::Event, errors::StorageError>;
    async fn update_event(
        &self,
        event_id: String,
        event: storage::EventUpdate,
    ) -> CustomResult<storage::Event, errors::StorageError>;
}

#[async_trait::async_trait]
impl EventInterface for Store {
    async fn insert_event(
        &self,
        event: storage::EventNew,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        event.insert(&conn).await.map_err(Into::into).into_report()
    }
    async fn update_event(
        &self,
        event_id: String,
        event: storage::EventUpdate,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Event::update(&conn, &event_id, event)
            .await
            .map_err(Into::into)
            .into_report()
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
    async fn update_event(
        &self,
        _event_id: String,
        _event: storage::EventUpdate,
    ) -> CustomResult<storage::Event, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
