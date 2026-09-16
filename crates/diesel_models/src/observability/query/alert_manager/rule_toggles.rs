use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::prelude::*;
use error_stack::report;

use crate::{
    errors::DatabaseError,
    observability::{
        alert_manager::rule_toggles::{RuleToggle, RuleToggleNew},
        schema::alert_rule_toggles::{self, dsl},
    },
    DatabaseConnectionWithContext, StorageResult,
};

impl RuleToggle {
    pub async fn list(conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Vec<Self>> {
        dsl::alert_rule_toggles
            .order(dsl::rule_id)
            .select(Self::as_select())
            .load_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}

impl RuleToggleNew {
    pub async fn upsert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<RuleToggle> {
        diesel::insert_into(alert_rule_toggles::table)
            .values(self)
            .on_conflict(dsl::rule_id)
            .do_update()
            .set((
                dsl::is_enabled.eq(diesel::upsert::excluded(dsl::is_enabled)),
                dsl::updated_by.eq(diesel::upsert::excluded(dsl::updated_by)),
                dsl::last_updated_at.eq(diesel::upsert::excluded(dsl::last_updated_at)),
            ))
            .returning(RuleToggle::as_returning())
            .get_result_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}
