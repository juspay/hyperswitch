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
    ) -> CustomResult<storage::GenericLinkS, errors::StorageError>;

    async fn insert_generic_link(
        &self,
        _generic_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::GenericLinkS, errors::StorageError>;
}

#[async_trait::async_trait]
impl GenericLinkInterface for Store {
    #[instrument(skip_all)]
    async fn find_generic_link_by_link_id(
        &self,
        link_id: &str,
    ) -> CustomResult<storage::GenericLinkS, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::GenericLink::find_generic_link_by_link_id(&conn, link_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn insert_generic_link(
        &self,
        generic_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::GenericLinkS, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        generic_link
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl GenericLinkInterface for MockDb {
    async fn insert_generic_link(
        &self,
        _generic_link: storage::GenericLinkNew,
    ) -> CustomResult<storage::GenericLinkS, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_generic_link_by_link_id(
        &self,
        _generic_link_id: &str,
    ) -> CustomResult<storage::GenericLinkS, errors::StorageError> {
        // TODO: Implement function for `MockDb`x
        Err(errors::StorageError::MockDbError)?
    }
}
