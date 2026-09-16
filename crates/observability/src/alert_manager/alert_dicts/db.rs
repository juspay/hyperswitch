//! Storage operations on `alerts_dicts`.

use async_bb8_diesel::AsyncConnection;
use diesel_models::{observability::alert_manager::alert_dicts as storage, StorageResult};

use crate::{
    db::{Store, TransactionError},
    domain_models::alert_manager::alert_dicts::domain_models,
};

/// r-apps' `OFFSET 2`: the live row plus one older version.
const VERSIONS_KEPT: i64 = 2;

/// Storage operations on the mappers dictionary.
#[async_trait::async_trait]
pub trait AlertsDictsInterface {
    /// Disable the live row for `(name, key_)`, insert the new one as the live row, and keep only
    /// the newest [`VERSIONS_KEPT`] versions. All in one transaction.
    async fn insert_alert_dict_version(
        &self,
        new: domain_models::AlertsDictsNew,
    ) -> StorageResult<domain_models::AlertsDicts>;

    /// Find one dictionary entry by id, any version.
    async fn find_alert_dict_by_id(
        &self,
        id: uuid::Uuid,
    ) -> StorageResult<domain_models::AlertsDicts>;

    /// List dictionary entries matching the given filter.
    async fn list_alert_dicts_by_filter(
        &self,
        filter: domain_models::AlertsDictsFilter,
    ) -> StorageResult<Vec<domain_models::AlertsDicts>>;

    /// Delete one dictionary entry by id, any version. Never re-enables another version.
    async fn delete_alert_dict_by_id(&self, id: uuid::Uuid) -> StorageResult<bool>;
}

#[async_trait::async_trait]
impl AlertsDictsInterface for Store {
    async fn insert_alert_dict_version(
        &self,
        new: domain_models::AlertsDictsNew,
    ) -> StorageResult<domain_models::AlertsDicts> {
        let connection = self.connection().await?;
        let (name, key_, new) = (
            new.name.clone(),
            new.key_.clone(),
            storage::AlertsDictsNew::from(new),
        );
        // The closure's handle is the same connection, so these run in the transaction.
        let connection = &connection;

        connection
            .raw_connection()
            .transaction_async(move |_| async move {
                storage::AlertsDicts::demote_enabled_by_name_key(connection, &name, &key_).await?;
                let stored = new.insert(connection).await?;
                let superseded = storage::AlertsDicts::find_superseded_ids_by_name_key(
                    connection,
                    &name,
                    &key_,
                    VERSIONS_KEPT,
                )
                .await?;
                storage::AlertsDicts::delete_by_ids(connection, superseded).await?;

                Ok::<_, TransactionError>(stored)
            })
            .await
            .map(domain_models::AlertsDicts::from)
            .map_err(|TransactionError(report)| report)
    }

    async fn find_alert_dict_by_id(
        &self,
        id: uuid::Uuid,
    ) -> StorageResult<domain_models::AlertsDicts> {
        let connection = self.connection().await?;

        storage::AlertsDicts::find_by_id(&connection, id)
            .await
            .map(domain_models::AlertsDicts::from)
    }

    async fn list_alert_dicts_by_filter(
        &self,
        filter: domain_models::AlertsDictsFilter,
    ) -> StorageResult<Vec<domain_models::AlertsDicts>> {
        let connection = self.connection().await?;

        storage::AlertsDicts::list_by_filter(
            &connection,
            filter.name,
            filter.key_,
            filter.is_enabled,
        )
        .await
        .map(|rows| {
            rows.into_iter()
                .map(domain_models::AlertsDicts::from)
                .collect()
        })
    }

    async fn delete_alert_dict_by_id(&self, id: uuid::Uuid) -> StorageResult<bool> {
        let connection = self.connection().await?;

        storage::AlertsDicts::delete_by_id(&connection, id).await
    }
}
