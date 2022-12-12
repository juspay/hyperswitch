use error_stack::IntoReport;

use super::MockDb;
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage::{Event, EventNew},
};

#[async_trait::async_trait]
pub trait EventInterface {
    async fn insert_event(&self, event: EventNew) -> CustomResult<Event, errors::StorageError>;
}

#[async_trait::async_trait]
impl EventInterface for super::Store {
    async fn insert_event(&self, event: EventNew) -> CustomResult<Event, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        event.insert(&conn).await.map_err(Into::into).into_report()
    }
}

#[async_trait::async_trait]
impl EventInterface for MockDb {
    async fn insert_event(&self, _event: EventNew) -> CustomResult<Event, errors::StorageError> {
        todo!()
    }
}
