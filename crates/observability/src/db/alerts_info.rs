//! Storage operations on `alerts_info`.

use diesel_models::{observability::alerts_info as storage, StorageResult};

use crate::{db::Store, domain_models::alerts_info as domain_models};

/// Storage operations on alert definitions.
#[async_trait::async_trait]
pub trait AlertsInfoInterface {
    /// Store a new alert definition and return it as the database saved it, defaults included.
    async fn insert_alert_info(
        &self,
        new: domain_models::AlertsInfoNew,
    ) -> StorageResult<domain_models::AlertsInfo>;

    /// The most recently updated alert definition for `(name, product)`, whatever `is_enabled`
    /// is set to, or `None` if no such alert is defined.
    async fn find_alert_info_by_name_product(
        &self,
        name: &str,
        product: &str,
    ) -> StorageResult<Option<domain_models::AlertsInfo>>;
}

#[async_trait::async_trait]
impl AlertsInfoInterface for Store {
    async fn insert_alert_info(
        &self,
        new: domain_models::AlertsInfoNew,
    ) -> StorageResult<domain_models::AlertsInfo> {
        let connection = self.connection().await?;

        storage::AlertsInfoNew::from(new)
            .insert(&connection)
            .await
            .map(domain_models::AlertsInfo::from)
    }

    async fn find_alert_info_by_name_product(
        &self,
        name: &str,
        product: &str,
    ) -> StorageResult<Option<domain_models::AlertsInfo>> {
        let connection = self.connection().await?;

        storage::AlertsInfo::find_latest_by_name_product(&connection, name, product)
            .await
            .map(|row| row.map(domain_models::AlertsInfo::from))
    }
}
