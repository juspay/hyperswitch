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
        merchant_id: &common_utils::id_type::MerchantId,
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
        merchant_id: &common_utils::id_type::MerchantId,
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
        merchant_id: &common_utils::id_type::MerchantId,
        created_after: time::PrimitiveDateTime,
        created_before: time::PrimitiveDateTime,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        use async_bb8_diesel::AsyncRunQueryDsl;
        use diesel::{debug_query, pg::Pg, QueryDsl};
        use error_stack::ResultExt;
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

        query = Self::apply_filters(
            query,
            None,
            (dsl::created_at, created_after, created_before),
            limit,
            offset,
        );

        logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        track_database_call::<Self, _, _>(query.get_results_async(conn), DatabaseOperation::Filter)
            .await
            .change_context(DatabaseError::Others) // Query returns empty Vec when no records are found
            .attach_printable("Error filtering events by constraints")
    }

    pub async fn list_by_merchant_id_initial_attempt_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
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

    pub async fn list_initial_attempts_by_profile_id_primary_object_id(
        conn: &PgPooledConn,
        profile_id: &common_utils::id_type::ProfileId,
        primary_object_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::event_id
                .nullable()
                .eq(dsl::initial_attempt_id) // Filter initial attempts only
                .and(dsl::business_profile_id.eq(profile_id.to_owned()))
                .and(dsl::primary_object_id.eq(primary_object_id.to_owned())),
            None,
            None,
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn list_initial_attempts_by_profile_id_constraints(
        conn: &PgPooledConn,
        profile_id: &common_utils::id_type::ProfileId,
        created_after: time::PrimitiveDateTime,
        created_before: time::PrimitiveDateTime,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        use async_bb8_diesel::AsyncRunQueryDsl;
        use diesel::{debug_query, pg::Pg, QueryDsl};
        use error_stack::ResultExt;
        use router_env::logger;

        use super::generics::db_metrics::{track_database_call, DatabaseOperation};
        use crate::errors::DatabaseError;

        let mut query = Self::table()
            .filter(
                dsl::event_id
                    .nullable()
                    .eq(dsl::initial_attempt_id) // Filter initial attempts only
                    .and(dsl::business_profile_id.eq(profile_id.to_owned())),
            )
            .order(dsl::created_at.desc())
            .into_boxed();

        query = Self::apply_filters(
            query,
            None,
            (dsl::created_at, created_after, created_before),
            limit,
            offset,
        );

        logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        track_database_call::<Self, _, _>(query.get_results_async(conn), DatabaseOperation::Filter)
            .await
            .change_context(DatabaseError::Others) // Query returns empty Vec when no records are found
            .attach_printable("Error filtering events by constraints")
    }

    pub async fn list_by_profile_id_initial_attempt_id(
        conn: &PgPooledConn,
        profile_id: &common_utils::id_type::ProfileId,
        initial_attempt_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::business_profile_id
                .eq(profile_id.to_owned())
                .and(dsl::initial_attempt_id.eq(initial_attempt_id.to_owned())),
            None,
            None,
            Some(dsl::created_at.desc()),
        )
        .await
    }

    pub async fn update_by_merchant_id_event_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
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

    fn apply_filters<T>(
        mut query: T,
        profile_id: Option<common_utils::id_type::ProfileId>,
        from_to: (
            dsl::created_at,
            time::PrimitiveDateTime,
            time::PrimitiveDateTime,
        ),
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> T
    where
        T: diesel::query_dsl::methods::LimitDsl<Output = T>
            + diesel::query_dsl::methods::OffsetDsl<Output = T>,
        T: diesel::query_dsl::methods::FilterDsl<
            diesel::dsl::GtEq<dsl::created_at, time::PrimitiveDateTime>,
            Output = T,
        >,
        T: diesel::query_dsl::methods::FilterDsl<
            diesel::dsl::LtEq<dsl::created_at, time::PrimitiveDateTime>,
            Output = T,
        >,
        T: diesel::query_dsl::methods::FilterDsl<
            diesel::dsl::Eq<dsl::business_profile_id, common_utils::id_type::ProfileId>,
            Output = T,
        >,
    {
        if let Some(profile_id) = profile_id {
            query = query.filter(dsl::business_profile_id.eq(profile_id));
        }

        query = query
            .filter(from_to.0.ge(from_to.1))
            .filter(from_to.0.le(from_to.2));

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        query
    }

    pub async fn count_initial_attempts_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: Option<common_utils::id_type::ProfileId>,
        created_after: time::PrimitiveDateTime,
        created_before: time::PrimitiveDateTime,
    ) -> StorageResult<i64> {
        use async_bb8_diesel::AsyncRunQueryDsl;
        use diesel::{debug_query, pg::Pg, QueryDsl};
        use error_stack::ResultExt;
        use router_env::logger;

        use super::generics::db_metrics::{track_database_call, DatabaseOperation};
        use crate::errors::DatabaseError;

        let mut query = Self::table()
            .count()
            .filter(
                dsl::event_id
                    .nullable()
                    .eq(dsl::initial_attempt_id) // Filter initial attempts only
                    .and(dsl::merchant_id.eq(merchant_id.to_owned())),
            )
            .into_boxed();

        query = Self::apply_filters(
            query,
            profile_id,
            (dsl::created_at, created_after, created_before),
            None,
            None,
        );

        logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        track_database_call::<Self, _, _>(
            query.get_result_async::<i64>(conn),
            DatabaseOperation::Count,
        )
        .await
        .change_context(DatabaseError::Others) // Query returns empty Vec when no records are found
        .attach_printable("Error counting events by constraints")
    }
}
