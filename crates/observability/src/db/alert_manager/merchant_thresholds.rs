//! Storage operations on `merchant_thresholds`.

use diesel_models::{observability::alert_manager::merchant_thresholds as storage, StorageResult};

use crate::{db::Store, domain_models::alert_manager::merchant_thresholds as domain_models};

/// Storage operations on per-merchant threshold overrides.
#[async_trait::async_trait]
pub trait MerchantThresholdsInterface {
    /// List rows matching the given filter.
    async fn list_merchant_thresholds_by_filter(
        &self,
        filter: domain_models::MerchantThresholdsFilter,
    ) -> StorageResult<Vec<domain_models::MerchantThresholds>>;

    /// Insert a new override, or, on a conflict of `(name, product, merchant_id, profile_id,
    /// is_enabled, author)`, set only the non-key columns the caller actually sent.
    async fn upsert_merchant_threshold(
        &self,
        new: domain_models::MerchantThresholdsNew,
    ) -> StorageResult<Vec<domain_models::MerchantThresholds>>;

    /// Set or clear columns on every row matching the given keys.
    async fn update_merchant_thresholds_by_filter(
        &self,
        filter: domain_models::MerchantThresholdsKeyFilter,
        update: domain_models::MerchantThresholdsUpdate,
    ) -> StorageResult<Vec<domain_models::MerchantThresholds>>;

    /// Delete one override by id.
    async fn delete_merchant_threshold_by_id(
        &self,
        id: String,
    ) -> StorageResult<domain_models::MerchantThresholds>;

    /// Delete every row matching the given keys.
    async fn delete_merchant_thresholds_by_filter(
        &self,
        filter: domain_models::MerchantThresholdsKeyFilter,
    ) -> StorageResult<Vec<domain_models::MerchantThresholds>>;
}

#[async_trait::async_trait]
impl MerchantThresholdsInterface for Store {
    async fn list_merchant_thresholds_by_filter(
        &self,
        filter: domain_models::MerchantThresholdsFilter,
    ) -> StorageResult<Vec<domain_models::MerchantThresholds>> {
        let connection = self.connection().await?;

        storage::MerchantThresholds::list_by_filter(&connection, filter.into())
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(domain_models::MerchantThresholds::from)
                    .collect()
            })
    }

    async fn upsert_merchant_threshold(
        &self,
        new: domain_models::MerchantThresholdsNew,
    ) -> StorageResult<Vec<domain_models::MerchantThresholds>> {
        let connection = self.connection().await?;
        let on_conflict = new.conflict_update().map(|update| update.into_storage().0);

        storage::MerchantThresholdsNew::from(new)
            .upsert(&connection, on_conflict)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(domain_models::MerchantThresholds::from)
                    .collect()
            })
    }

    async fn update_merchant_thresholds_by_filter(
        &self,
        filter: domain_models::MerchantThresholdsKeyFilter,
        update: domain_models::MerchantThresholdsUpdate,
    ) -> StorageResult<Vec<domain_models::MerchantThresholds>> {
        let connection = self.connection().await?;
        let (changeset, merge) = update.into_storage();

        storage::MerchantThresholds::update_by_key_filter(
            &connection,
            filter.into(),
            changeset,
            merge,
        )
        .await
        .map(|rows| {
            rows.into_iter()
                .map(domain_models::MerchantThresholds::from)
                .collect()
        })
    }

    async fn delete_merchant_threshold_by_id(
        &self,
        id: String,
    ) -> StorageResult<domain_models::MerchantThresholds> {
        let connection = self.connection().await?;

        storage::MerchantThresholds::delete_by_id(&connection, id)
            .await
            .map(domain_models::MerchantThresholds::from)
    }

    async fn delete_merchant_thresholds_by_filter(
        &self,
        filter: domain_models::MerchantThresholdsKeyFilter,
    ) -> StorageResult<Vec<domain_models::MerchantThresholds>> {
        let connection = self.connection().await?;

        storage::MerchantThresholds::delete_by_key_filter(&connection, filter.into())
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(domain_models::MerchantThresholds::from)
                    .collect()
            })
    }
}
