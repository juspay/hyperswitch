use diesel_models::{
    errors::DatabaseError, observability::alert_manager::alert_rules as storage, StorageResult,
};
use time::PrimitiveDateTime;

use crate::{db::Store, domain_models::alert_rules as domain};

#[async_trait::async_trait]
pub trait AlertRuleInterface {
    async fn insert_alert_rule(
        &self,
        new: domain::AlertRuleNew,
    ) -> StorageResult<domain::AlertRule>;

    async fn update_alert_rule(
        &self,
        id: String,
        update: domain::AlertRuleUpdate,
    ) -> StorageResult<domain::AlertRule>;

    async fn list_alert_rules(&self) -> StorageResult<Vec<domain::AlertRule>>;

    async fn list_due_alert_rules(
        &self,
        now: PrimitiveDateTime,
    ) -> StorageResult<Vec<domain::AlertRule>>;

    async fn set_alert_rule_next_run_at(
        &self,
        id: String,
        next_run_at: PrimitiveDateTime,
    ) -> StorageResult<bool>;

    async fn set_alert_rule_last_evaluated_at(
        &self,
        id: String,
        last_evaluated_at: PrimitiveDateTime,
    ) -> StorageResult<bool>;
}

async fn set_evaluator_field(
    store: &Store,
    id: String,
    update: storage::AlertRuleUpdate,
) -> StorageResult<bool> {
    match storage::AlertRule::update_by_id(&store.connection().await?, id, update).await {
        Ok(_) => Ok(true),
        Err(error) if matches!(error.current_context(), DatabaseError::NotFound) => Ok(false),
        Err(error) => Err(error),
    }
}

#[async_trait::async_trait]
impl AlertRuleInterface for Store {
    async fn insert_alert_rule(
        &self,
        new: domain::AlertRuleNew,
    ) -> StorageResult<domain::AlertRule> {
        storage::AlertRuleNew::from(new)
            .insert(&self.connection().await?)
            .await
            .map(domain::AlertRule::from)
    }

    async fn update_alert_rule(
        &self,
        id: String,
        update: domain::AlertRuleUpdate,
    ) -> StorageResult<domain::AlertRule> {
        storage::AlertRule::update_by_id(
            &self.connection().await?,
            id,
            storage::AlertRuleUpdate::from(update),
        )
        .await
        .map(domain::AlertRule::from)
    }

    async fn list_alert_rules(&self) -> StorageResult<Vec<domain::AlertRule>> {
        storage::AlertRule::list(&self.connection().await?)
            .await
            .map(|rows| rows.into_iter().map(domain::AlertRule::from).collect())
    }

    async fn list_due_alert_rules(
        &self,
        now: PrimitiveDateTime,
    ) -> StorageResult<Vec<domain::AlertRule>> {
        storage::AlertRule::list_due(&self.connection().await?, now)
            .await
            .map(|rows| rows.into_iter().map(domain::AlertRule::from).collect())
    }

    async fn set_alert_rule_next_run_at(
        &self,
        id: String,
        next_run_at: PrimitiveDateTime,
    ) -> StorageResult<bool> {
        set_evaluator_field(self, id, storage::AlertRuleUpdate::NextRun { next_run_at }).await
    }

    async fn set_alert_rule_last_evaluated_at(
        &self,
        id: String,
        last_evaluated_at: PrimitiveDateTime,
    ) -> StorageResult<bool> {
        set_evaluator_field(
            self,
            id,
            storage::AlertRuleUpdate::Evaluated { last_evaluated_at },
        )
        .await
    }
}
