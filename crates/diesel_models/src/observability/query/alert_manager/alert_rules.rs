use async_bb8_diesel::AsyncRunQueryDsl;
use common_types::alert_rules::stored::AlertRuleStatus;
use diesel::prelude::*;
use error_stack::report;
use time::PrimitiveDateTime;

use crate::{
    errors::DatabaseError,
    observability::{
        alert_manager::alert_rules::{AlertRule, AlertRuleNew},
        schema::alert_rules::{self, dsl},
    },
    DatabaseConnectionWithContext, StorageResult,
};

impl AlertRule {
    pub async fn list(conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Vec<Self>> {
        dsl::alert_rules
            .order(dsl::id)
            .select(Self::as_select())
            .load_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }

    pub async fn find(
        conn: &DatabaseConnectionWithContext<'_>,
        id: &str,
    ) -> StorageResult<Option<Self>> {
        dsl::alert_rules
            .filter(dsl::id.eq(id.to_owned()))
            .select(Self::as_select())
            .first_async(conn.raw_connection())
            .await
            .optional()
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }

    pub async fn list_active(conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Vec<Self>> {
        dsl::alert_rules
            .filter(dsl::status.eq(AlertRuleStatus::Active))
            .order(dsl::id)
            .select(Self::as_select())
            .load_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }

    /// Active rules whose `next_run_at` has passed.
    pub async fn list_due(
        conn: &DatabaseConnectionWithContext<'_>,
        now: PrimitiveDateTime,
    ) -> StorageResult<Vec<Self>> {
        dsl::alert_rules
            .filter(dsl::status.eq(AlertRuleStatus::Active))
            .filter(dsl::next_run_at.le(now))
            .order(dsl::id)
            .select(Self::as_select())
            .load_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }

    /// Move an active rule's `next_run_at` from `seen` to `next`. `false` when the rule changed
    /// since it was read (another caller claimed it, or it was replaced or deactivated).
    pub async fn claim(
        conn: &DatabaseConnectionWithContext<'_>,
        id: &str,
        seen: PrimitiveDateTime,
        next: PrimitiveDateTime,
    ) -> StorageResult<bool> {
        diesel::update(
            dsl::alert_rules
                .filter(dsl::id.eq(id.to_owned()))
                .filter(dsl::status.eq(AlertRuleStatus::Active))
                .filter(dsl::next_run_at.eq(seen)),
        )
        .set(dsl::next_run_at.eq(next))
        .execute_async(conn.raw_connection())
        .await
        .map(|updated| updated == 1)
        .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }

    /// `false` when no rule has this id.
    pub async fn delete(conn: &DatabaseConnectionWithContext<'_>, id: &str) -> StorageResult<bool> {
        diesel::delete(dsl::alert_rules.filter(dsl::id.eq(id.to_owned())))
            .execute_async(conn.raw_connection())
            .await
            .map(|deleted| deleted > 0)
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}

impl AlertRuleNew {
    /// Create the rule, or replace everything but `created_at` of an existing one.
    pub async fn upsert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<AlertRule> {
        diesel::insert_into(alert_rules::table)
            .values(self)
            .on_conflict(dsl::id)
            .do_update()
            .set((
                dsl::name.eq(diesel::upsert::excluded(dsl::name)),
                dsl::status.eq(diesel::upsert::excluded(dsl::status)),
                dsl::alerts_configuration.eq(diesel::upsert::excluded(dsl::alerts_configuration)),
                dsl::next_run_at.eq(diesel::upsert::excluded(dsl::next_run_at)),
                dsl::last_updated_at.eq(diesel::upsert::excluded(dsl::last_updated_at)),
            ))
            .returning(AlertRule::as_returning())
            .get_result_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}
