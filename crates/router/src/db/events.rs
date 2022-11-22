use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    services::Store,
    types::storage::{Event, EventNew},
};

#[async_trait::async_trait]
pub trait IEvent {
    async fn insert_event(&self, event: EventNew) -> CustomResult<Event, errors::StorageError>;
}

#[async_trait::async_trait]
impl IEvent for Store {
    async fn insert_event(&self, event: EventNew) -> CustomResult<Event, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        event.insert(&conn).await
    }
}
