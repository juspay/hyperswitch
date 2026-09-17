//! Storage operations for success-rate threshold overrides.

use diesel_models::{observability::alert_manager::thresholds as storage, StorageResult};

use crate::{db::Store, domain_models::thresholds as domain};

#[async_trait::async_trait]
pub trait ThresholdsInterface {
    async fn list_threshold_overrides(&self) -> StorageResult<Vec<domain::ThresholdOverride>>;

    async fn upsert_threshold_override(
        &self,
        new: domain::ThresholdOverrideNew,
        max_active_rules: i64,
    ) -> StorageResult<domain::ThresholdUpsertOutcome>;

    async fn delete_threshold_override(
        &self,
        tombstone: domain::ThresholdOverrideNew,
    ) -> StorageResult<domain::ThresholdOverride>;
}

#[async_trait::async_trait]
impl ThresholdsInterface for Store {
    async fn list_threshold_overrides(&self) -> StorageResult<Vec<domain::ThresholdOverride>> {
        let connection = self.connection().await?;
        storage::ThresholdOverride::list_active(&connection)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(domain::ThresholdOverride::from)
                    .collect()
            })
    }

    async fn upsert_threshold_override(
        &self,
        new: domain::ThresholdOverrideNew,
        max_active_rules: i64,
    ) -> StorageResult<domain::ThresholdUpsertOutcome> {
        match storage::ThresholdOverrideNew::from(new)
            .upsert_with_limit(&self.connection().await?, max_active_rules)
            .await?
        {
            storage::ThresholdUpsertOutcome::Stored(row) => Ok(
                domain::ThresholdUpsertOutcome::Stored(domain::ThresholdOverride::from(row)),
            ),
            storage::ThresholdUpsertOutcome::ActiveRuleLimitReached => {
                Ok(domain::ThresholdUpsertOutcome::ActiveRuleLimitReached)
            }
        }
    }

    async fn delete_threshold_override(
        &self,
        tombstone: domain::ThresholdOverrideNew,
    ) -> StorageResult<domain::ThresholdOverride> {
        storage::ThresholdOverrideNew::from(tombstone)
            .tombstone(&self.connection().await?)
            .await
            .map(domain::ThresholdOverride::from)
    }
}
