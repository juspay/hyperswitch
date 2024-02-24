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
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Event> {
        generics::generic_insert(conn, self).await
    }
}

impl Event {
    #[instrument(skip(conn))]
    pub async fn find_by_event_id(conn: &PgPooledConn, event_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::event_id.eq(event_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
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
