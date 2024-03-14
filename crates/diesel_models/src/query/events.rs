use diesel::{associations::HasTable, ExpressionMethods};

use super::generics;
use crate::{
    events::{Event, EventNew, EventUpdateInternal},
    schema::events::dsl,
    PgPooledConn, StorageResult,
};

impl EventNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Event> {
        generics::generic_insert(conn, self).await
    }
}

impl Event {
    pub async fn find_by_event_id(conn: &PgPooledConn, event_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::event_id.eq(event_id.to_owned()),
        )
        .await
    }

    pub async fn update(
        conn: &PgPooledConn,
        event_id: &str,
        event: EventUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(conn, dsl::event_id.eq(event_id.to_owned()), event)
        .await
    }
}
