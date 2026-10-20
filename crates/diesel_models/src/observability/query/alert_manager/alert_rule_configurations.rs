use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, prelude::*};
use error_stack::report;

use crate::{
    errors::DatabaseError,
    observability::{
        alert_manager::alert_rule_configurations::{
            AlertRuleConfiguration, AlertRuleConfigurationNew, AlertRuleConfigurationUpdate,
            AlertRuleConfigurationUpdateInternal,
        },
        schema::alert_rule_configurations::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

impl AlertRuleConfigurationNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<AlertRuleConfiguration> {
        generics::generic_insert(conn, self).await
    }
}

impl AlertRuleConfiguration {
    pub async fn update_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: String,
        update: AlertRuleConfigurationUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            id,
            AlertRuleConfigurationUpdateInternal::from(update),
        )
        .await
    }

    pub async fn list(conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Vec<Self>> {
        dsl::alert_rule_configurations
            .order(dsl::name)
            .select(Self::as_select())
            .load_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}
