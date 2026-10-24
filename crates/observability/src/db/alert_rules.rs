//! Storage operations for alert rules.

use diesel_models::{observability::alert_manager::alert_rules as storage, StorageResult};
use time::PrimitiveDateTime;

use crate::{db::Store, domain_models::alert_rules as domain};

#[async_trait::async_trait]
pub trait AlertRulesInterface {
    async fn list_alert_rules(&self) -> StorageResult<Vec<domain::AlertRule>>;

    async fn find_alert_rule(&self, id: &str) -> StorageResult<Option<domain::AlertRule>>;

    async fn upsert_alert_rule(
        &self,
        new: domain::AlertRuleNew,
    ) -> StorageResult<domain::AlertRule>;

    /// `false` when no rule has this id.
    async fn delete_alert_rule(&self, id: &str) -> StorageResult<bool>;

    async fn list_active_alert_rules(&self) -> StorageResult<Vec<domain::AlertRule>>;

    /// Active rules whose `next_run_at` has passed.
    async fn list_due_alert_rules(
        &self,
        now: PrimitiveDateTime,
    ) -> StorageResult<Vec<domain::AlertRule>>;

    /// Move an active rule's `next_run_at` from `seen` to `next`. `false` when the rule changed
    /// since it was read, so whoever changed it first runs it.
    async fn claim_alert_rule(
        &self,
        id: &str,
        seen: PrimitiveDateTime,
        next: PrimitiveDateTime,
    ) -> StorageResult<bool>;
}

#[async_trait::async_trait]
impl AlertRulesInterface for Store {
    async fn list_alert_rules(&self) -> StorageResult<Vec<domain::AlertRule>> {
        storage::AlertRule::list(&self.connection().await?)
            .await
            .map(|rows| rows.into_iter().map(domain::AlertRule::from).collect())
    }

    async fn find_alert_rule(&self, id: &str) -> StorageResult<Option<domain::AlertRule>> {
        storage::AlertRule::find(&self.connection().await?, id)
            .await
            .map(|row| row.map(domain::AlertRule::from))
    }

    async fn upsert_alert_rule(
        &self,
        new: domain::AlertRuleNew,
    ) -> StorageResult<domain::AlertRule> {
        storage::AlertRuleNew::from(new)
            .upsert(&self.connection().await?)
            .await
            .map(domain::AlertRule::from)
    }

    async fn delete_alert_rule(&self, id: &str) -> StorageResult<bool> {
        storage::AlertRule::delete(&self.connection().await?, id).await
    }

    async fn list_active_alert_rules(&self) -> StorageResult<Vec<domain::AlertRule>> {
        storage::AlertRule::list_active(&self.connection().await?)
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

    async fn claim_alert_rule(
        &self,
        id: &str,
        seen: PrimitiveDateTime,
        next: PrimitiveDateTime,
    ) -> StorageResult<bool> {
        storage::AlertRule::claim(&self.connection().await?, id, seen, next).await
    }
}
