use error_stack::report;
use router_env::{instrument, tracing};

use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::MockDb,
    services::Store,
    types::storage,
};

#[async_trait::async_trait]
pub trait GenericLinkInterface {
    async fn find_generic_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::GenericLinkState, errors::StorageError>;

    async fn find_pm_collect_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::PaymentMethodCollectLink, errors::StorageError>;

    async fn find_payout_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::PayoutLink, errors::StorageError>;

    async fn insert_generic_link(
        &self,
        _generic_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::GenericLinkState, errors::StorageError>;

    async fn insert_pm_collect_link(
        &self,
        _pm_collect_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::PaymentMethodCollectLink, errors::StorageError>;

    async fn insert_payout_link(
        &self,
        _payout_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::PayoutLink, errors::StorageError>;

    async fn update_payout_link(
        &self,
        payout_link: storage::PayoutLink,
        payout_link_update: storage::PayoutLinkUpdate,
    ) -> CustomResult<storage::PayoutLink, errors::StorageError>;
}

#[async_trait::async_trait]
impl GenericLinkInterface for Store {
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

#[async_trait::async_trait]
impl GenericLinkInterface for MockDb {
    async fn find_generic_link_by_link_id(
        &self,
        _generic_link_id: &str,
    ) -> CustomResult<storage::GenericLinkState, errors::StorageError> {
        // TODO: Implement function for `MockDb`x
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_pm_collect_link_by_link_id(
        &self,
        _generic_link_id: &str,
    ) -> CustomResult<storage::PaymentMethodCollectLink, errors::StorageError> {
        // TODO: Implement function for `MockDb`x
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_payout_link_by_link_id(
        &self,
        _generic_link_id: &str,
    ) -> CustomResult<storage::PayoutLink, errors::StorageError> {
        // TODO: Implement function for `MockDb`x
        Err(errors::StorageError::MockDbError)?
    }

    async fn insert_generic_link(
        &self,
        _generic_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::GenericLinkState, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn insert_pm_collect_link(
        &self,
        _pm_collect_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::PaymentMethodCollectLink, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn insert_payout_link(
        &self,
        _pm_collect_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::PayoutLink, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_payout_link(
        &self,
        _payout_link: storage::PayoutLink,
        _payout_link_update: storage::PayoutLinkUpdate,
    ) -> CustomResult<storage::PayoutLink, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
