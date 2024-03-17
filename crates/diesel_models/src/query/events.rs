use diesel::{
    associations::HasTable, BoolExpressionMethods, ExpressionMethods, NullableExpressionMethods,
};

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

    pub async fn list_initial_attempts_by_merchant_id_primary_object_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        primary_object_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::event_id
                .nullable()
                .eq(dsl::initial_attempt_id) // Filter initial attempts only
                .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                .and(dsl::primary_object_id.eq(primary_object_id.to_owned())),
            None,
            None,
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn list_initial_attempts_by_merchant_id_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        created_after: Option<time::PrimitiveDateTime>,
        created_before: Option<time::PrimitiveDateTime>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        use async_bb8_diesel::AsyncRunQueryDsl;
        use diesel::{debug_query, pg::Pg, QueryDsl};
        use error_stack::{IntoReport, ResultExt};
        use router_env::logger;

        use super::generics::db_metrics::{track_database_call, DatabaseOperation};
        use crate::errors::DatabaseError;

        let mut query = Self::table()
            .filter(
                dsl::event_id
                    .nullable()
                    .eq(dsl::initial_attempt_id) // Filter initial attempts only
                    .and(dsl::merchant_id.eq(merchant_id.to_owned())),
            )
            .order(dsl::created_at.desc())
            .into_boxed();

        if let Some(created_after) = created_after {
            query = query.filter(dsl::created_at.ge(created_after));
        }

        if let Some(created_before) = created_before {
            query = query.filter(dsl::created_at.le(created_before));
        }

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        track_database_call::<Self, _, _>(query.get_results_async(conn), DatabaseOperation::Filter)
            .await
            .into_report()
            .change_context(DatabaseError::Others) // Query returns empty Vec when no records are found
            .attach_printable("Error filtering events by constraints")
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
