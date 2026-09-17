use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::prelude::*;
use error_stack::report;

use crate::{
    errors::DatabaseError,
    observability::{
        alert_manager::metadata::{AlertMetadataChangeset, AlertMetadataEntry, AlertMetadataNew},
        schema::alert_metadata::{self, dsl},
    },
    DatabaseConnectionWithContext, StorageResult,
};

impl AlertMetadataEntry {
    pub async fn list(conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Vec<Self>> {
        dsl::alert_metadata
            .order(dsl::id)
            .select(Self::as_select())
            .load_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}

impl AlertMetadataNew {
    pub async fn patch(
        self,
        changeset: AlertMetadataChangeset,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<AlertMetadataEntry> {
        diesel::insert_into(alert_metadata::table)
            .values(self)
            .on_conflict(dsl::id)
            .do_update()
            .set((
                changeset,
                dsl::last_updated_at.eq(diesel::upsert::excluded(dsl::last_updated_at)),
            ))
            .returning(AlertMetadataEntry::as_returning())
            .get_result_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}
