use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::prelude::*;
use error_stack::report;

use crate::{
    errors::DatabaseError,
    observability::{
        alert_manager::dictionary::{DictionaryEntry, DictionaryEntryNew},
        schema::alert_dictionary::{self, dsl},
    },
    DatabaseConnectionWithContext, StorageResult,
};

impl DictionaryEntry {
    pub async fn list(conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Vec<Self>> {
        dsl::alert_dictionary
            .order((dsl::name, dsl::key_))
            .select(Self::as_select())
            .load_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}

impl DictionaryEntryNew {
    pub async fn upsert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<DictionaryEntry> {
        diesel::insert_into(alert_dictionary::table)
            .values(self)
            .on_conflict((dsl::name, dsl::key_))
            .do_update()
            .set((
                dsl::product.eq(diesel::upsert::excluded(dsl::product)),
                dsl::values_.eq(diesel::upsert::excluded(dsl::values_)),
                dsl::metadata.eq(diesel::upsert::excluded(dsl::metadata)),
                dsl::updated_by.eq(diesel::upsert::excluded(dsl::updated_by)),
                dsl::last_updated_at.eq(diesel::upsert::excluded(dsl::last_updated_at)),
            ))
            .returning(DictionaryEntry::as_returning())
            .get_result_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}
