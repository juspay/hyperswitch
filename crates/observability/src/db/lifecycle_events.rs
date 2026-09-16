//! Storage operations for alert lifecycle episode state.

use diesel_models::{observability::alert_manager::lifecycle_events as storage, StorageResult};
use time::PrimitiveDateTime;

use crate::{db::Store, domain_models::lifecycle_events as domain};

#[async_trait::async_trait]
pub trait LifecycleEventsInterface {
    async fn list_lifecycle_events(
        &self,
        from: Option<PrimitiveDateTime>,
        to: Option<PrimitiveDateTime>,
    ) -> StorageResult<Vec<domain::LifecycleEvent>>;

    async fn replace_lifecycle_events(
        &self,
        batch: domain::LifecycleEventsBatch,
    ) -> StorageResult<usize>;
}

#[async_trait::async_trait]
impl LifecycleEventsInterface for Store {
    async fn list_lifecycle_events(
        &self,
        from: Option<PrimitiveDateTime>,
        to: Option<PrimitiveDateTime>,
    ) -> StorageResult<Vec<domain::LifecycleEvent>> {
        storage::LifecycleEvent::list_overlapping(&self.connection().await?, from, to)
            .await
            .map(|rows| rows.into_iter().map(domain::LifecycleEvent::from).collect())
    }

    async fn replace_lifecycle_events(
        &self,
        batch: domain::LifecycleEventsBatch,
    ) -> StorageResult<usize> {
        let rows = batch
            .events
            .into_iter()
            .map(storage::LifecycleEventNew::from)
            .collect();
        storage::LifecycleEvent::replace_batch_and_cleanup(
            rows,
            batch.retention_cutoff,
            &self.connection().await?,
        )
        .await
    }
}
