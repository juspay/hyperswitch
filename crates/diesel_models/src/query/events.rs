use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    events::{Event, EventNew, EventUpdate, EventUpdateInternal},
    schema::events::dsl,
    PgPooledConn, StorageResult,
};

impl EventNew {
    #[instrument(skip(conn))]
        /// Inserts a new event into the database using the provided PostgreSQL connection.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled PostgreSQL connection
    ///
    /// # Returns
    ///
    /// The result of the database insertion operation, wrapped in a `StorageResult`
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Event> {
        generics::generic_insert(conn, self).await
    }
}

impl Event {
    #[instrument(skip(conn))]
        /// Asynchronously updates an event in the database based on the provided event_id and EventUpdate. Returns a result indicating the success of the operation.
    pub async fn update(
        conn: &PgPooledConn,
        event_id: &str,
        event: EventUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::event_id.eq(event_id.to_owned()),
            EventUpdateInternal::from(event),
        )
        .await
    }
}
