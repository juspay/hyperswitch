use common_utils::errors::CustomResult;
use diesel_models::locker_mock_up as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::locker_mock_up::LockerMockUpInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> LockerMockUpInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn find_locker_by_card_id(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::LockerMockUp::find_by_card_id(&conn, card_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn insert_locker_mock_up(
        &self,
        new: storage::LockerMockUpNew,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_locker_mock_up(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::LockerMockUp::delete_by_card_id(&conn, card_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}
