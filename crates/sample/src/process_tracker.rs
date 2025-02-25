use common_utils::errors::CustomResult;
pub use diesel_models as storage;
use diesel_models::enums as storage_enums;
// use error_stack::{report, ResultExt};
// use storage_impl::{connection, errors, mock_db::MockDb};
use time::PrimitiveDateTime;

// use crate::{metrics, scheduler::Store};

#[async_trait::async_trait]
pub trait ProcessTrackerInterface: Send + Sync {
    type Error;
    async fn reinitialize_limbo_processes(
        &self,
        ids: Vec<String>,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<usize, Self::Error>;

    async fn find_process_by_id(
        &self,
        id: &str,
    ) -> CustomResult<Option<storage::ProcessTracker>, Self::Error>;

    async fn update_process(
        &self,
        this: storage::ProcessTracker,
        process: storage::ProcessTrackerUpdate,
    ) -> CustomResult<storage::ProcessTracker, Self::Error>;

    async fn process_tracker_update_process_status_by_ids(
        &self,
        task_ids: Vec<String>,
        task_update: storage::ProcessTrackerUpdate,
    ) -> CustomResult<usize, Self::Error>;

    async fn insert_process(
        &self,
        new: storage::ProcessTrackerNew,
    ) -> CustomResult<storage::ProcessTracker, Self::Error>;

    async fn reset_process(
        &self,
        this: storage::ProcessTracker,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<(), Self::Error>;

    async fn retry_process(
        &self,
        this: storage::ProcessTracker,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<(), Self::Error>;

    async fn finish_process_with_business_status(
        &self,
        this: storage::ProcessTracker,
        business_status: &'static str,
    ) -> CustomResult<(), Self::Error>;

    async fn find_processes_by_time_status(
        &self,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        status: storage_enums::ProcessTrackerStatus,
        limit: Option<i64>,
    ) -> CustomResult<Vec<storage::ProcessTracker>, Self::Error>;
}