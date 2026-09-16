//! Storage operations on `alerts_info`.

use diesel_models::{observability::alert_manager::alert_info as storage, StorageResult};

use crate::{db::Store, domain_models::alert_manager::alert_info::domain_models};

/// Storage operations on alert definitions.
#[async_trait::async_trait]
pub trait AlertsInfoInterface {
    /// Store a new alert definition and return it as the database saved it, defaults included.
    async fn insert_alert_info(
        &self,
        new: domain_models::AlertsInfoNew,
    ) -> StorageResult<domain_models::AlertsInfo>;

    async fn list_alert_info(
        &self,
        name: Option<String>,
        product: Option<String>,
        is_enabled: Option<bool>,
    ) -> StorageResult<Vec<domain_models::AlertsInfo>>;

    async fn find_alert_info_by_id(&self, id: String) -> StorageResult<domain_models::AlertsInfo>;

    async fn update_alert_info_by_id(
        &self,
        id: String,
        update: diesel_models::observability::alert_manager::alert_info::AlertsInfoUpdate,
    ) -> StorageResult<domain_models::AlertsInfo>;

    async fn delete_alert_info_by_id(&self, id: String) -> StorageResult<bool>;
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

    async fn list_alert_info(
        &self,
        name: Option<String>,
        product: Option<String>,
        is_enabled: Option<bool>,
    ) -> StorageResult<Vec<domain_models::AlertsInfo>> {
        let connection = self.connection().await?;
        storage::AlertsInfo::list_by_filter(&connection, name, product, is_enabled)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(domain_models::AlertsInfo::from)
                    .collect()
            })
    }

    async fn find_alert_info_by_id(&self, id: String) -> StorageResult<domain_models::AlertsInfo> {
        let connection = self.connection().await?;
        storage::AlertsInfo::find_by_id(&connection, id)
            .await
            .map(domain_models::AlertsInfo::from)
    }

    async fn update_alert_info_by_id(
        &self,
        id: String,
        update: diesel_models::observability::alert_manager::alert_info::AlertsInfoUpdate,
    ) -> StorageResult<domain_models::AlertsInfo> {
        let connection = self.connection().await?;
        storage::AlertsInfo::update_by_id(&connection, id, update)
            .await
            .map(domain_models::AlertsInfo::from)
    }

    async fn delete_alert_info_by_id(&self, id: String) -> StorageResult<bool> {
        let connection = self.connection().await?;
        storage::AlertsInfo::delete_by_id(&connection, id).await
    }
}
