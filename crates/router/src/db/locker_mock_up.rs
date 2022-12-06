use super::MockDb;
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait LockerMockUpInterface {
    async fn find_locker_by_card_id(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError>;

    async fn insert_locker_mock_up(
        &self,
        new: storage::LockerMockUpNew,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError>;
}

#[async_trait::async_trait]
impl LockerMockUpInterface for super::Store {
    async fn find_locker_by_card_id(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        storage::LockerMockUp::find_by_card_id(&conn, card_id).await
    }

    async fn insert_locker_mock_up(
        &self,
        new: storage::LockerMockUpNew,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        new.insert(&conn).await
    }
}

#[async_trait::async_trait]
impl LockerMockUpInterface for MockDb {
    async fn find_locker_by_card_id(
        &self,
        _card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        todo!()
    }

    async fn insert_locker_mock_up(
        &self,
        _new: storage::LockerMockUpNew,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        todo!()
    }
}
