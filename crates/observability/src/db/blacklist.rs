//! Storage operations for alert blacklist state.

use diesel_models::{observability::alert_manager::blacklist as storage, StorageResult};

use crate::{db::Store, domain_models::blacklist as domain};

#[async_trait::async_trait]
pub trait BlacklistInterface {
    async fn list_blacklist_entries(&self) -> StorageResult<Vec<domain::BlacklistEntry>>;

    async fn upsert_blacklist_entry(
        &self,
        new: domain::BlacklistEntryNew,
        max_active_rules: i64,
    ) -> StorageResult<domain::BlacklistUpsertOutcome>;

    async fn delete_blacklist_entry(
        &self,
        tombstone: domain::BlacklistEntryNew,
    ) -> StorageResult<domain::BlacklistEntry>;
}

#[async_trait::async_trait]
impl BlacklistInterface for Store {
    async fn list_blacklist_entries(&self) -> StorageResult<Vec<domain::BlacklistEntry>> {
        let connection = self.connection().await?;
        storage::BlacklistEntry::list_active(&connection)
            .await
            .map(|rows| rows.into_iter().map(domain::BlacklistEntry::from).collect())
    }

    async fn upsert_blacklist_entry(
        &self,
        new: domain::BlacklistEntryNew,
        max_active_rules: i64,
    ) -> StorageResult<domain::BlacklistUpsertOutcome> {
        match storage::BlacklistEntryNew::from(new)
            .upsert_with_limit(&self.connection().await?, max_active_rules)
            .await?
        {
            storage::BlacklistUpsertOutcome::Stored(row) => Ok(
                domain::BlacklistUpsertOutcome::Stored(domain::BlacklistEntry::from(row)),
            ),
            storage::BlacklistUpsertOutcome::ActiveRuleLimitReached => {
                Ok(domain::BlacklistUpsertOutcome::ActiveRuleLimitReached)
            }
        }
    }

    async fn delete_blacklist_entry(
        &self,
        tombstone: domain::BlacklistEntryNew,
    ) -> StorageResult<domain::BlacklistEntry> {
        storage::BlacklistEntryNew::from(tombstone)
            .tombstone(&self.connection().await?)
            .await
            .map(domain::BlacklistEntry::from)
    }
}
