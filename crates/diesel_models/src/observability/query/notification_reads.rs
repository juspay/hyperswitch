use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, upsert::excluded, ExpressionMethods};
use error_stack::ResultExt;

use crate::{
    errors,
    observability::{
        notification_reads::{NotificationReads, NotificationReadsNew},
        schema::notification_reads::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

impl NotificationReadsNew {
    pub async fn upsert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<NotificationReads> {
        let query = diesel::insert_into(<NotificationReads as HasTable>::table())
            .values(self)
            .on_conflict(dsl::user_name)
            .do_update()
            .set(dsl::last_read_at.eq(excluded(dsl::last_read_at)));

        generics::db_metrics::track_database_call::<<NotificationReads as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Insert,
            query.get_result_async(conn.raw_connection()),
        )
        .await
        .map_err(|e| error_stack::report!(e))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to upsert into notification_reads")
    }
}

impl NotificationReads {
    pub async fn find_by_user_name(
        conn: &DatabaseConnectionWithContext<'_>,
        user_name: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, user_name.to_owned())
            .await
    }
}
