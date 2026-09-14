use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, query_dsl::methods::FilterDsl, BoolExpressionMethods, ExpressionMethods,
};
use error_stack::ResultExt;

use crate::{
    errors,
    observability::{
        merchants_alert_external_dimension::{
            MerchantsAlertExternalDimension, MerchantsAlertExternalDimensionNew,
        },
        schema::merchants_alert_external_dimension::dsl,
    },
    query::{
        generics,
        observability::{advisory_xact_lock, DIMENSIONS_LOCK_NAMESPACE},
    },
    DatabaseConnectionWithContext, StorageResult,
};

impl MerchantsAlertExternalDimensionNew {
    pub async fn bulk_insert(
        conn: &DatabaseConnectionWithContext<'_>,
        rows: Vec<Self>,
    ) -> StorageResult<usize> {
        if rows.is_empty() {
            return Ok(0);
        }

        let query = diesel::insert_into(<MerchantsAlertExternalDimension as HasTable>::table())
            .values(rows);

        generics::db_metrics::track_database_call::<
            <MerchantsAlertExternalDimension as HasTable>::Table,
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
        .attach_printable("Failed to insert alert dimension rows")
    }
}

impl MerchantsAlertExternalDimension {
    pub async fn lock_announcement(
        conn: &DatabaseConnectionWithContext<'_>,
        announcement: uuid::Uuid,
    ) -> StorageResult<()> {
        advisory_xact_lock::<<Self as HasTable>::Table>(
            conn,
            DIMENSIONS_LOCK_NAMESPACE,
            &announcement.to_string(),
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
            Some((
                dsl::dimension_key.asc(),
                dsl::dimension_value.asc(),
                dsl::id_merchant_table.asc(),
            )),
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
        .attach_printable("Failed to delete alert dimension rows")
    }
}
