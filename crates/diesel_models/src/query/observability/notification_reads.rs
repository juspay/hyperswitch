use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, upsert::excluded, ExpressionMethods};
use error_stack::{report, ResultExt};

use crate::{
    errors,
    observability::{notification_reads::NotificationRead, schema::notification_reads::dsl},
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

impl NotificationRead {
    pub async fn find_by_user_name(
        conn: &DatabaseConnectionWithContext<'_>,
        user_name: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            user_name.to_owned(),
        )
        .await
    }

    pub async fn upsert(self, conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Self> {
        let query = diesel::insert_into(<Self as HasTable>::table())
            .values(self)
            .on_conflict(dsl::user_name)
            .do_update()
            .set(dsl::last_read_at.eq(excluded(dsl::last_read_at)));

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Insert,
            query.get_result_async(conn.raw_connection()),
        )
        .await
        .map_err(|error| report!(error).change_context(errors::DatabaseError::Others))
        .attach_printable("Error while saving a notification read watermark")
    }
}
