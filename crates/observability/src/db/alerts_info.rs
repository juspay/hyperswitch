//! Storage operations on `alerts_info`.

use diesel_models::{observability::alerts_info as storage, StorageResult};

use crate::{db::Store, domain::alerts_info as domain};

/// Storage operations on alert definitions.
#[async_trait::async_trait]
pub trait AlertsInfoInterface {
    /// Store a new alert definition and return it as the database saved it, defaults included.
    async fn insert_alert_info(
        &self,
        new: domain::AlertsInfoNew,
    ) -> StorageResult<domain::AlertsInfo>;
}

#[async_trait::async_trait]
impl AlertsInfoInterface for Store {
    async fn insert_alert_info(
        &self,
        new: domain::AlertsInfoNew,
    ) -> StorageResult<domain::AlertsInfo> {
        let connection = self.connection().await?;

        storage::AlertsInfoNew::from(new)
            .insert(&connection)
            .await
            .map(domain::AlertsInfo::from)
    }
}
