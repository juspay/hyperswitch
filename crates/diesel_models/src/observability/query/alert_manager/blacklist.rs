use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
use diesel::{dsl::count_star, prelude::*};
use error_stack::report;

use crate::{
    errors::DatabaseError,
    observability::{
        alert_manager::blacklist::{BlacklistEntry, BlacklistEntryNew, BlacklistUpsertOutcome},
        schema::{
            alert_blacklist::{self, dsl},
            alert_blacklist_write_lock,
        },
    },
    DatabaseConnectionWithContext, StorageResult,
};

impl BlacklistEntry {
    pub async fn list_active(conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Vec<Self>> {
        dsl::alert_blacklist
            .filter(dsl::is_deleted.eq(false))
            .order((dsl::merchant_id, dsl::profile_id))
            .select(Self::as_select())
            .load_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}

impl BlacklistEntryNew {
    pub async fn upsert_with_limit(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
        max_active_rules: i64,
    ) -> StorageResult<BlacklistUpsertOutcome> {
        conn.raw_connection()
            .transaction_async(move |connection| async move {
                // Serialize every cap check and write. Locking the singleton before reading
                // also makes a waiting transaction's subsequent reads observe the commit that
                // released the lock under PostgreSQL's default READ COMMITTED isolation.
                alert_blacklist_write_lock::table
                    .select(alert_blacklist_write_lock::lock_key)
                    .for_update()
                    .first_async::<i16>(&connection)
                    .await?;

                let already_active = dsl::alert_blacklist
                    .filter(dsl::rule_id.eq(self.rule_id.clone()))
                    .filter(dsl::merchant_id.eq(self.merchant_id.clone()))
                    .filter(dsl::profile_id.eq(self.profile_id.clone()))
                    .select(dsl::is_deleted)
                    .first_async::<bool>(&connection)
                    .await
                    .optional()?
                    .is_some_and(|deleted| !deleted);

                if max_active_rules > 0 && !already_active {
                    let active_count = dsl::alert_blacklist
                        .filter(dsl::is_deleted.eq(false))
                        .select(count_star())
                        .first_async::<i64>(&connection)
                        .await?;
                    if active_count >= max_active_rules {
                        return Ok(BlacklistUpsertOutcome::ActiveRuleLimitReached);
                    }
                }

                diesel::insert_into(alert_blacklist::table)
                    .values(self)
                    .on_conflict((dsl::rule_id, dsl::merchant_id, dsl::profile_id))
                    .do_update()
                    .set((
                        dsl::reason.eq(diesel::upsert::excluded(dsl::reason)),
                        dsl::created_by.eq(diesel::upsert::excluded(dsl::created_by)),
                        dsl::last_updated_at.eq(diesel::upsert::excluded(dsl::last_updated_at)),
                        dsl::is_deleted.eq(false),
                    ))
                    .returning(BlacklistEntry::as_returning())
                    .get_result_async(&connection)
                    .await
                    .map(BlacklistUpsertOutcome::Stored)
            })
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }

    pub async fn tombstone(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<BlacklistEntry> {
        diesel::insert_into(alert_blacklist::table)
            .values(self)
            .on_conflict((dsl::rule_id, dsl::merchant_id, dsl::profile_id))
            .do_update()
            .set((
                dsl::reason.eq(diesel::upsert::excluded(dsl::reason)),
                dsl::created_by.eq(diesel::upsert::excluded(dsl::created_by)),
                dsl::last_updated_at.eq(diesel::upsert::excluded(dsl::last_updated_at)),
                dsl::is_deleted.eq(true),
            ))
            .returning(BlacklistEntry::as_returning())
            .get_result_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}
