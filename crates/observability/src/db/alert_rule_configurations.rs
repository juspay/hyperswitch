use diesel_models::{
    errors::DatabaseError, observability::alert_manager::alert_rule_configurations as storage,
    StorageResult,
};
use error_stack::report;

use crate::{db::Store, domain_models::alert_rule_configurations as domain_models};

#[async_trait::async_trait]
pub trait AlertRuleConfigurationInterface {
    async fn insert_alert_rule_configuration(
        &self,
        new: domain_models::AlertRuleConfigurationNew,
    ) -> StorageResult<storage::AlertRuleConfiguration>;

    async fn update_alert_rule_configuration(
        &self,
        id: String,
        update: domain_models::AlertRuleConfigurationUpdate,
    ) -> StorageResult<storage::AlertRuleConfiguration>;

    async fn list_alert_rule_configurations(
        &self,
    ) -> StorageResult<Vec<storage::AlertRuleConfiguration>>;
}

#[async_trait::async_trait]
impl AlertRuleConfigurationInterface for Store {
    async fn insert_alert_rule_configuration(
        &self,
        new: domain_models::AlertRuleConfigurationNew,
    ) -> StorageResult<storage::AlertRuleConfiguration> {
        let new = storage::AlertRuleConfigurationNew::try_from(new)
            .map_err(|error| report!(error).change_context(DatabaseError::Others))?;
        new.insert(&self.connection().await?).await
    }

    async fn update_alert_rule_configuration(
        &self,
        id: String,
        update: domain_models::AlertRuleConfigurationUpdate,
    ) -> StorageResult<storage::AlertRuleConfiguration> {
        let update = storage::AlertRuleConfigurationUpdate::try_from(update)
            .map_err(|error| report!(error).change_context(DatabaseError::Others))?;
        storage::AlertRuleConfiguration::update_by_id(&self.connection().await?, id, update).await
    }

    async fn list_alert_rule_configurations(
        &self,
    ) -> StorageResult<Vec<storage::AlertRuleConfiguration>> {
        storage::AlertRuleConfiguration::list(&self.connection().await?).await
    }
}
