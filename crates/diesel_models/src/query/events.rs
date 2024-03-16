use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

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
    pub async fn find_by_merchant_id_event_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        event_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::event_id.eq(event_id.to_owned())),
        )
        .await
    }

    pub async fn list_by_merchant_id_initial_attempt_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        initial_attempt_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::initial_attempt_id.eq(initial_attempt_id.to_owned())),
            None,
            None,
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn update_by_merchant_id_event_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        event_id: &str,
        event: EventUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::event_id.eq(event_id.to_owned())),
            event,
        )
        .await
    }
}
