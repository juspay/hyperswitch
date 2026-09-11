use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use error_stack::{report, ResultExt};

use crate::{
    errors,
    observability::{
        merchants_alert_external_dimension::DimensionInstance,
        schema::merchants_alert_external_dimension::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

impl DimensionInstance {
    pub async fn list_for_announcement(
        conn: &DatabaseConnectionWithContext<'_>,
        announcement: uuid::Uuid,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
            conn,
            dsl::id.eq(announcement),
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

    pub async fn delete_for_announcement(
        conn: &DatabaseConnectionWithContext<'_>,
        announcement: uuid::Uuid,
    ) -> StorageResult<usize> {
        let query = diesel::delete(<Self as HasTable>::table().filter(dsl::id.eq(announcement)));

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Delete,
            query.execute_async(conn.raw_connection()),
        )
        .await
        .map_err(|error| report!(error).change_context(errors::DatabaseError::Others))
        .attach_printable("Error while removing alert dimension rows")
    }

    // One statement for the whole batch:
    pub async fn insert_all(
        conn: &DatabaseConnectionWithContext<'_>,
        rows: Vec<Self>,
    ) -> StorageResult<usize> {
        if rows.is_empty() {
            return Ok(0);
        }

        let query = diesel::insert_into(<Self as HasTable>::table()).values(rows);

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Insert,
            query.execute_async(conn.raw_connection()),
        )
        .await
        .map_err(|error| report!(error).change_context(errors::DatabaseError::Others))
        .attach_printable("Error while saving alert dimension rows")
    }
}
