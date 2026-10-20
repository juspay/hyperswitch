use diesel_models::{
    errors::DatabaseError, observability::alert_manager::alert_rule_configurations as storage,
    StorageResult,
};
use error_stack::report;
use time::PrimitiveDateTime;

use crate::{db::Store, domain_models::alert_rule_configurations as domain_models};

#[async_trait::async_trait]
pub trait AlertRuleConfigurationInterface {
    async fn insert_alert_rule_configuration(
        &self,
        new: domain_models::AlertRuleConfigurationNew,
    ) -> StorageResult<domain_models::AlertRuleConfiguration>;

    async fn update_alert_rule_configuration(
        &self,
        id: String,
        update: domain_models::AlertRuleConfigurationUpdate,
    ) -> StorageResult<domain_models::AlertRuleConfiguration>;

    async fn list_alert_rule_configurations(
        &self,
    ) -> StorageResult<domain_models::AlertRuleConfigurationListing>;

    async fn list_due_alert_rule_configurations(
        &self,
        now: PrimitiveDateTime,
    ) -> StorageResult<domain_models::AlertRuleConfigurationListing>;

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

fn convert_row(
    row: storage::AlertRuleConfiguration,
) -> StorageResult<domain_models::AlertRuleConfiguration> {
    domain_models::AlertRuleConfiguration::try_from(row).map_err(|error| {
        let context = error.to_string();
        report!(error)
            .change_context(DatabaseError::Others)
            .attach_printable(context)
    })
}

fn convert_listing(
    rows: Vec<storage::AlertRuleConfiguration>,
) -> domain_models::AlertRuleConfigurationListing {
    let mut rules = Vec::with_capacity(rows.len());
    let mut unreadable_ids = Vec::new();
    for row in rows {
        let id = row.id.clone();
        match domain_models::AlertRuleConfiguration::try_from(row) {
            Ok(rule) => rules.push(rule),
            Err(error) => {
                crate::logger::error!(rule_id = %id, error = ?error, "Failed to deserialize alert rule configuration");
                unreadable_ids.push(id);
            }
        }
    }
    domain_models::AlertRuleConfigurationListing {
        rules,
        unreadable_ids,
    }
}

#[async_trait::async_trait]
impl AlertRuleConfigurationInterface for Store {
    async fn insert_alert_rule_configuration(
        &self,
        new: domain_models::AlertRuleConfigurationNew,
    ) -> StorageResult<domain_models::AlertRuleConfiguration> {
        let new = storage::AlertRuleConfigurationNew::try_from(new).map_err(|error| {
            let context = error.to_string();
            report!(error)
                .change_context(DatabaseError::Others)
                .attach_printable(context)
        })?;
        convert_row(new.insert(&self.connection().await?).await?)
    }

    async fn update_alert_rule_configuration(
        &self,
        id: String,
        update: domain_models::AlertRuleConfigurationUpdate,
    ) -> StorageResult<domain_models::AlertRuleConfiguration> {
        let update = update.into_storage(&id).map_err(|error| {
            let context = error.to_string();
            report!(error)
                .change_context(DatabaseError::Others)
                .attach_printable(context)
        })?;
        convert_row(
            storage::AlertRuleConfiguration::update_by_id(&self.connection().await?, id, update)
                .await?,
        )
    }

    async fn list_alert_rule_configurations(
        &self,
    ) -> StorageResult<domain_models::AlertRuleConfigurationListing> {
        Ok(convert_listing(
            storage::AlertRuleConfiguration::list(&self.connection().await?).await?,
        ))
    }

    async fn list_due_alert_rule_configurations(
        &self,
        now: PrimitiveDateTime,
    ) -> StorageResult<domain_models::AlertRuleConfigurationListing> {
        Ok(convert_listing(
            storage::AlertRuleConfiguration::list_due(&self.connection().await?, now).await?,
        ))
    }

    async fn set_alert_rule_next_run_at(
        &self,
        id: String,
        next_run_at: PrimitiveDateTime,
    ) -> StorageResult<bool> {
        match storage::AlertRuleConfiguration::update_by_id(
            &self.connection().await?,
            id,
            storage::AlertRuleConfigurationUpdate::NextRun { next_run_at },
        )
        .await
        {
            Ok(_) => Ok(true),
            Err(error) if matches!(error.current_context(), DatabaseError::NotFound) => Ok(false),
            Err(error) => Err(error),
        }
    }

    async fn set_alert_rule_last_evaluated_at(
        &self,
        id: String,
        last_evaluated_at: PrimitiveDateTime,
    ) -> StorageResult<bool> {
        match storage::AlertRuleConfiguration::update_by_id(
            &self.connection().await?,
            id,
            storage::AlertRuleConfigurationUpdate::Evaluated { last_evaluated_at },
        )
        .await
        {
            Ok(_) => Ok(true),
            Err(error) if matches!(error.current_context(), DatabaseError::NotFound) => Ok(false),
            Err(error) => Err(error),
        }
    }
}
