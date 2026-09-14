use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable,
    query_dsl::methods::FilterDsl,
    sql_types::{Integer, Text},
    BoolExpressionMethods, ExpressionMethods,
};
use error_stack::ResultExt;

use crate::{
    errors,
    observability::{
        merchants_alert_external::{MerchantsAlertExternal, MerchantsAlertExternalNew},
        schema::merchants_alert_external::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

const INSTANCES_LOCK_NAMESPACE: i32 = 23_405;

impl MerchantsAlertExternalNew {
    pub async fn bulk_insert(
        conn: &DatabaseConnectionWithContext<'_>,
        rows: Vec<Self>,
    ) -> StorageResult<usize> {
        if rows.is_empty() {
            return Ok(0);
        }

        let query = diesel::insert_into(<MerchantsAlertExternal as HasTable>::table()).values(rows);

        generics::db_metrics::track_database_call::<
            <MerchantsAlertExternal as HasTable>::Table,
            _,
            _,
        >(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Insert,
            query.execute_async(conn.raw_connection()),
        )
        .await
        .map_err(|e| error_stack::report!(e))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to insert merchant alert instances")
    }
}

impl MerchantsAlertExternal {
    pub async fn lock_announcement(
        conn: &DatabaseConnectionWithContext<'_>,
        announcement: uuid::Uuid,
    ) -> StorageResult<()> {
        let query = diesel::sql_query("SELECT pg_advisory_xact_lock($1, hashtext($2))")
            .bind::<Integer, _>(INSTANCES_LOCK_NAMESPACE)
            .bind::<Text, _>(announcement.to_string());

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
        .attach_printable("Failed to lock the merchant alert instances of an announcement")
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
            Some((dsl::merchant_id.asc(), dsl::id_merchant_table.asc())),
        )
        .await
    }

    pub async fn delete_by_channel_and_announcement(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: &str,
        announcement: uuid::Uuid,
    ) -> StorageResult<usize> {
        let query = diesel::delete(
            <Self as HasTable>::table().filter(
                dsl::channel
                    .eq(channel.to_owned())
                    .and(dsl::id.eq(announcement)),
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
        .attach_printable("Failed to delete merchant alert instances")
    }
}
