use async_bb8_diesel::AsyncRunQueryDsl;
use common_enums::AlertRuleStatus;
use diesel::{associations::HasTable, prelude::*};
use error_stack::report;
use time::PrimitiveDateTime;

use crate::{
    errors::DatabaseError,
    observability::{
        alert_manager::alert_rules::{
            AlertRule, AlertRuleNew, AlertRuleUpdate, AlertRuleUpdateInternal,
        },
        schema::alert_rules::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

impl AlertRuleNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<AlertRule> {
        generics::generic_insert(conn, self).await
    }
}

impl AlertRule {
    pub async fn update_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: String,
        update: AlertRuleUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            id,
            AlertRuleUpdateInternal::from(update),
        )
        .await
    }

    pub async fn list(conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Vec<Self>> {
        dsl::alert_rules
            .order(dsl::id)
            .select(Self::as_select())
            .load_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }

    pub async fn list_due(
        conn: &DatabaseConnectionWithContext<'_>,
        now: PrimitiveDateTime,
    ) -> StorageResult<Vec<Self>> {
        dsl::alert_rules
            .filter(
                dsl::status
                    .eq(AlertRuleStatus::Active)
                    .and(dsl::next_run_at.le(now)),
            )
            .order(dsl::next_run_at)
            .select(Self::as_select())
            .load_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}
