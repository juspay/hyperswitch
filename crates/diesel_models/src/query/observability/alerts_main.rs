use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use error_stack::{report, ResultExt};

use crate::{
    errors,
    observability::{alerts_main::AnnouncementRow, schema::alerts_main::dsl},
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

impl AnnouncementRow {
    pub async fn insert(self, conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Self> {
        generics::generic_insert::<<Self as HasTable>::Table, Self, Self>(conn, self).await
    }

    pub async fn existing_ids(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: &str,
        ids: Vec<uuid::Uuid>,
    ) -> StorageResult<Vec<uuid::Uuid>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let query = <Self as HasTable>::table()
            .select(dsl::id)
            .filter(dsl::channel.eq(channel.to_owned()))
            .filter(dsl::id.eq_any(ids));

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Filter,
            query.load_async::<uuid::Uuid>(conn.raw_connection()),
        )
        .await
        .map_err(|error| report!(error).change_context(errors::DatabaseError::Others))
        .attach_printable("Error while checking which announcements exist")
    }
}
