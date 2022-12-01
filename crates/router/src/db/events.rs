use super::{MockDb, Sqlx};
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    services::Store,
    types::storage::{Event, EventNew},
};

#[async_trait::async_trait]
pub trait EventInterface {
    async fn insert_event(&self, event: EventNew) -> CustomResult<Event, errors::StorageError>;
}

#[async_trait::async_trait]
impl EventInterface for Store {
    async fn insert_event(&self, event: EventNew) -> CustomResult<Event, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        event.insert(&conn).await
    }
}

#[async_trait::async_trait]
impl EventInterface for Sqlx {
    async fn insert_event(&self, event: EventNew) -> CustomResult<Event, errors::StorageError> {
        todo!()
    }
}

#[async_trait::async_trait]
impl EventInterface for MockDb {
    async fn insert_event(&self, event: EventNew) -> CustomResult<Event, errors::StorageError> {
        todo!()
    }
}
