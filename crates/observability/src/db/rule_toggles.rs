//! Storage operations for alert rule enable/disable state.

use diesel_models::{observability::alert_manager::rule_toggles as storage, StorageResult};

use crate::{db::Store, domain_models::rule_toggles as domain};

#[async_trait::async_trait]
pub trait RuleTogglesInterface {
    async fn list_rule_toggles(&self) -> StorageResult<Vec<domain::RuleToggle>>;

    async fn set_rule_toggle(
        &self,
        new: domain::RuleToggleNew,
    ) -> StorageResult<domain::RuleToggle>;
}

#[async_trait::async_trait]
impl RuleTogglesInterface for Store {
    async fn list_rule_toggles(&self) -> StorageResult<Vec<domain::RuleToggle>> {
        let connection = self.connection().await?;
        storage::RuleToggle::list(&connection)
            .await
            .map(|rows| rows.into_iter().map(domain::RuleToggle::from).collect())
    }

    async fn set_rule_toggle(
        &self,
        new: domain::RuleToggleNew,
    ) -> StorageResult<domain::RuleToggle> {
        storage::RuleToggleNew::from(new)
            .upsert(&self.connection().await?)
            .await
            .map(domain::RuleToggle::from)
    }
}
