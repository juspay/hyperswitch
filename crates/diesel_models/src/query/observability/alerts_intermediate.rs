use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable,
    query_dsl::methods::FilterDsl,
    sql_types::{Integer, Text},
    upsert::excluded,
    BoolExpressionMethods, ExpressionMethods, PgSortExpressionMethods,
};
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::{
    errors,
    observability::{
        alerts_intermediate::{
            AlertsIntermediate, AlertsIntermediateNew, AlertsIntermediateUpdate,
            AlertsIntermediateUpdateInternal,
        },
        schema::alerts_intermediate::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

const LIFECYCLE_LOCK_NAMESPACE: i32 = 23_404;

impl AlertsIntermediateNew {
    pub async fn bulk_upsert_within_channel(
        conn: &DatabaseConnectionWithContext<'_>,
        rows: Vec<Self>,
    ) -> StorageResult<usize> {
        let query = diesel::insert_into(<AlertsIntermediate as HasTable>::table())
            .values(rows)
            .on_conflict(dsl::id_intermediate)
            .do_update()
            .set((
                dsl::id.eq(excluded(dsl::id)),
                dsl::name.eq(excluded(dsl::name)),
                dsl::product.eq(excluded(dsl::product)),
                dsl::dimensions.eq(excluded(dsl::dimensions)),
                dsl::ts_slack.eq(excluded(dsl::ts_slack)),
                dsl::ts_alert.eq(excluded(dsl::ts_alert)),
                dsl::latest_ts_alert.eq(excluded(dsl::latest_ts_alert)),
                dsl::max_duration.eq(excluded(dsl::max_duration)),
                dsl::other_metrics.eq(excluded(dsl::other_metrics)),
                dsl::metadata.eq(excluded(dsl::metadata)),
                dsl::metadata_alert_details.eq(excluded(dsl::metadata_alert_details)),
                dsl::rca_metadata.eq(excluded(dsl::rca_metadata)),
                dsl::group_id.eq(excluded(dsl::group_id)),
                dsl::priority.eq(excluded(dsl::priority)),
                dsl::last_updated_at.eq(excluded(dsl::last_updated_at)),
                dsl::recovered_ts.eq(excluded(dsl::recovered_ts)),
            ))
            .filter(dsl::channel.eq(excluded(dsl::channel)));

        generics::db_metrics::track_database_call::<<AlertsIntermediate as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Insert,
            query.execute_async(conn.raw_connection()),
        )
        .await
        .map_err(|e| error_stack::report!(e))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to upsert lifecycle state rows")
    }
}

impl AlertsIntermediate {
    pub async fn lock_channel(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: &str,
    ) -> StorageResult<()> {
        let query = diesel::sql_query("SELECT pg_advisory_xact_lock($1, hashtext($2))")
            .bind::<Integer, _>(LIFECYCLE_LOCK_NAMESPACE)
            .bind::<Text, _>(channel.to_owned());

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Filter,
            query.execute_async(conn.raw_connection()),
        )
        .await
        .map(|_| ())
        .map_err(|e| error_stack::report!(e))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to lock the lifecycle state of a channel")
    }

    pub async fn list_by_channel(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::channel.eq(channel.to_owned()),
            None,
            None,
            Some((dsl::ts_alert.asc(), dsl::id_intermediate.asc())),
        )
        .await
    }

    pub async fn list_by_channel_and_announcement(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: &str,
        announcement: uuid::Uuid,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::channel
                .eq(channel.to_owned())
                .and(dsl::id.eq(announcement)),
            None,
            None,
            Some(dsl::id_intermediate.asc()),
        )
        .await
    }

    pub async fn update_by_id_intermediate(
        conn: &DatabaseConnectionWithContext<'_>,
        id_intermediate: uuid::Uuid,
        update: AlertsIntermediateUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::id_intermediate.eq(id_intermediate),
            AlertsIntermediateUpdateInternal::from(update),
        )
        .await
    }

    pub async fn find_latest_last_updated_at_by_channel(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: &str,
    ) -> StorageResult<Option<PrimitiveDateTime>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
            conn,
            dsl::channel.eq(channel.to_owned()),
            Some(1),
            None,
            Some(dsl::last_updated_at.desc().nulls_last()),
        )
        .await
        .map(|rows| rows.into_iter().next().and_then(|row| row.last_updated_at))
    }

    pub async fn delete_by_channel_excluding_ids(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: &str,
        ids: Vec<uuid::Uuid>,
    ) -> StorageResult<usize> {
        let query = diesel::delete(
            <Self as HasTable>::table().filter(
                dsl::channel
                    .eq(channel.to_owned())
                    .and(dsl::id_intermediate.ne_all(ids)),
            ),
        );

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Delete,
            query.execute_async(conn.raw_connection()),
        )
        .await
        .map_err(|e| error_stack::report!(e))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to delete lifecycle state rows")
    }
}
