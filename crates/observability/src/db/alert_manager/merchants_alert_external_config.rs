//! Storage operations on `merchants_alert_external_config`.

use diesel_models::{
    observability::alert_manager::merchants_alert_external_config as storage, StorageResult,
};

use crate::{
    db::Store, domain_models::alert_manager::merchants_alert_external_config as domain_models,
};

/// Storage operations on merchant alert delivery switches.
#[async_trait::async_trait]
pub trait MerchantsAlertExternalConfigInterface {
    /// Store a new merchant alert delivery switch and return it as the database saved it,
    /// defaults included.
    async fn insert_merchant_alert_external_config(
        &self,
        new: domain_models::MerchantsAlertExternalConfigNew,
    ) -> StorageResult<domain_models::MerchantsAlertExternalConfig>;

    /// Find one row by its `(name, product)` key.
    async fn find_merchant_alert_external_config_by_name_product(
        &self,
        name: &str,
        product: &str,
    ) -> StorageResult<domain_models::MerchantsAlertExternalConfig>;

    /// List rows matching the given filter.
    async fn list_merchant_alert_external_configs_by_filter(
        &self,
        filter: domain_models::MerchantsAlertExternalConfigListFilter,
    ) -> StorageResult<Vec<domain_models::MerchantsAlertExternalConfig>>;

    /// Change the given fields of one row.
    async fn update_merchant_alert_external_config_by_name_product(
        &self,
        name: &str,
        product: &str,
        update: domain_models::MerchantsAlertExternalConfigUpdate,
    ) -> StorageResult<domain_models::MerchantsAlertExternalConfig>;

    /// Remove one row by its `(name, product)` key.
    async fn delete_merchant_alert_external_config_by_name_product(
        &self,
        name: &str,
        product: &str,
    ) -> StorageResult<domain_models::MerchantsAlertExternalConfig>;
}

#[async_trait::async_trait]
impl MerchantsAlertExternalConfigInterface for Store {
    async fn insert_merchant_alert_external_config(
        &self,
        new: domain_models::MerchantsAlertExternalConfigNew,
    ) -> StorageResult<domain_models::MerchantsAlertExternalConfig> {
        let connection = self.connection().await?;

        storage::MerchantsAlertExternalConfigNew::from(new)
            .insert(&connection)
            .await
            .map(domain_models::MerchantsAlertExternalConfig::from)
    }

    async fn find_merchant_alert_external_config_by_name_product(
        &self,
        name: &str,
        product: &str,
    ) -> StorageResult<domain_models::MerchantsAlertExternalConfig> {
        let connection = self.connection().await?;

        storage::MerchantsAlertExternalConfig::find_by_name_product(&connection, name, product)
            .await
            .map(domain_models::MerchantsAlertExternalConfig::from)
    }

    async fn list_merchant_alert_external_configs_by_filter(
        &self,
        filter: domain_models::MerchantsAlertExternalConfigListFilter,
    ) -> StorageResult<Vec<domain_models::MerchantsAlertExternalConfig>> {
        let connection = self.connection().await?;

        storage::MerchantsAlertExternalConfig::list_by_filter(&connection, filter.into())
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(domain_models::MerchantsAlertExternalConfig::from)
                    .collect()
            })
    }

    async fn update_merchant_alert_external_config_by_name_product(
        &self,
        name: &str,
        product: &str,
        update: domain_models::MerchantsAlertExternalConfigUpdate,
    ) -> StorageResult<domain_models::MerchantsAlertExternalConfig> {
        let connection = self.connection().await?;

        storage::MerchantsAlertExternalConfig::update_by_name_product(
            &connection,
            name,
            product,
            update.into(),
        )
        .await
        .map(domain_models::MerchantsAlertExternalConfig::from)
    }

    async fn delete_merchant_alert_external_config_by_name_product(
        &self,
        name: &str,
        product: &str,
    ) -> StorageResult<domain_models::MerchantsAlertExternalConfig> {
        let connection = self.connection().await?;

        storage::MerchantsAlertExternalConfig::delete_by_name_product(&connection, name, product)
            .await
            .map(domain_models::MerchantsAlertExternalConfig::from)
    }
}
