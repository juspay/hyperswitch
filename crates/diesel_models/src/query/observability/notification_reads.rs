use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, sql_types::Timestamp, upsert::excluded, ExpressionMethods};
use error_stack::ResultExt;
use hyperswitch_masking::Secret;

use crate::{
    errors,
    observability::{
        notification_reads::{NotificationRead, NotificationReadNew},
        schema::notification_reads::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

diesel::define_sql_function! {
    fn greatest(left: Timestamp, right: Timestamp) -> Timestamp;
}

impl NotificationReadNew {
    pub async fn upsert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<NotificationRead> {
        let query = diesel::insert_into(<NotificationRead as HasTable>::table())
            .values(self)
            .on_conflict(dsl::user_name)
            .do_update()
            .set(dsl::last_read_at.eq(greatest(dsl::last_read_at, excluded(dsl::last_read_at))));

        generics::db_metrics::track_database_call::<<NotificationRead as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Insert,
            query.get_result_async(conn.raw_connection()),
        )
        .await
        .map_err(|error| error_stack::report!(error))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to upsert a notification watermark")
    }
}

impl NotificationRead {
    pub async fn find_by_user_name(
        conn: &DatabaseConnectionWithContext<'_>,
        user_name: Secret<String>,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(conn, user_name)
            .await
    }
}
