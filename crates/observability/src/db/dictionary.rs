//! Storage operations for alert dictionary state.

use diesel_models::{observability::alert_manager::dictionary as storage, StorageResult};

use crate::{db::Store, domain_models::dictionary as domain};

#[async_trait::async_trait]
pub trait DictionaryInterface {
    async fn list_dictionary_entries(&self) -> StorageResult<Vec<domain::DictionaryEntry>>;

    async fn upsert_dictionary_entry(
        &self,
        new: domain::DictionaryEntryNew,
    ) -> StorageResult<domain::DictionaryEntry>;
}

#[async_trait::async_trait]
impl DictionaryInterface for Store {
    async fn list_dictionary_entries(&self) -> StorageResult<Vec<domain::DictionaryEntry>> {
        let connection = self.connection().await?;
        storage::DictionaryEntry::list(&connection)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(domain::DictionaryEntry::from)
                    .collect()
            })
    }

    async fn upsert_dictionary_entry(
        &self,
        new: domain::DictionaryEntryNew,
    ) -> StorageResult<domain::DictionaryEntry> {
        storage::DictionaryEntryNew::from(new)
            .upsert(&self.connection().await?)
            .await
            .map(domain::DictionaryEntry::from)
    }
}
