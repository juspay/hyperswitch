//! Storage operations for per-alert metadata and snooze state.

use diesel_models::{observability::alert_manager::metadata as storage, StorageResult};

use crate::{db::Store, domain_models::metadata as domain};

#[async_trait::async_trait]
pub trait AlertMetadataInterface {
    async fn list_alert_metadata(&self) -> StorageResult<Vec<domain::AlertMetadataEntry>>;

    async fn patch_alert_metadata(
        &self,
        patch: domain::AlertMetadataPatch,
    ) -> StorageResult<domain::AlertMetadataEntry>;
}

#[async_trait::async_trait]
impl AlertMetadataInterface for Store {
    async fn list_alert_metadata(&self) -> StorageResult<Vec<domain::AlertMetadataEntry>> {
        storage::AlertMetadataEntry::list(&self.connection().await?)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(domain::AlertMetadataEntry::from)
                    .collect()
            })
    }

    async fn patch_alert_metadata(
        &self,
        patch: domain::AlertMetadataPatch,
    ) -> StorageResult<domain::AlertMetadataEntry> {
        let (new, changeset) = patch.into_storage();
        new.patch(changeset, &self.connection().await?)
            .await
            .map(domain::AlertMetadataEntry::from)
    }
}
