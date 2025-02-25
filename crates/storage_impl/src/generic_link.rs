use common_utils::errors::CustomResult;
use diesel_models::generic_link as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::generic_link::GenericLinkInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> GenericLinkInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn find_generic_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::GenericLinkState, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::GenericLink::find_generic_link_by_link_id(&conn, link_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_pm_collect_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::PaymentMethodCollectLink, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::GenericLink::find_pm_collect_link_by_link_id(&conn, link_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_payout_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::PayoutLink, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::GenericLink::find_payout_link_by_link_id(&conn, link_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn insert_generic_link(
        &self,
        generic_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::GenericLinkState, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        generic_link
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn insert_pm_collect_link(
        &self,
        pm_collect_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::PaymentMethodCollectLink, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        pm_collect_link
            .insert_pm_collect_link(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn insert_payout_link(
        &self,
        pm_collect_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::PayoutLink, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        pm_collect_link
            .insert_payout_link(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_payout_link(
        &self,
        payout_link: storage::PayoutLink,
        payout_link_update: storage::PayoutLinkUpdate,
    ) -> CustomResult<storage::PayoutLink, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        payout_link
            .update_payout_link(&conn, payout_link_update)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}
