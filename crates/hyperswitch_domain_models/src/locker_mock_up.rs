use common_utils::errors::CustomResult;
use diesel_models::locker_mock_up as storage;
pub use diesel_models::locker_mock_up::*;

#[async_trait::async_trait]
pub trait LockerMockUpInterface {
    type Error;
    async fn find_locker_by_card_id(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, Self::Error>;

    async fn insert_locker_mock_up(
        &self,
        new: storage::LockerMockUpNew,
    ) -> CustomResult<storage::LockerMockUp, Self::Error>;

    async fn delete_locker_mock_up(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, Self::Error>;
}