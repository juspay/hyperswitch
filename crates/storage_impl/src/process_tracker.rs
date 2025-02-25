use time::PrimitiveDateTime;

use common_utils::errors::CustomResult;
use diesel_models::{enums, process_tracker as storage};
use error_stack::{report, ResultExt};
// use router_env::{instrument, tracing};
use sample::process_tracker::ProcessTrackerInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> ProcessTrackerInterface for RouterStore<T> {
    type Error = errors::StorageError;

    async fn find_process_by_id(
        &self,
        id: &str,
    ) -> CustomResult<Option<storage::ProcessTracker>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::ProcessTracker::find_process_by_id(&conn, id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn reinitialize_limbo_processes(
        &self,
        ids: Vec<String>,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::ProcessTracker::reinitialize_limbo_processes(&conn, ids, schedule_time)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn find_processes_by_time_status(
        &self,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        status: enums::ProcessTrackerStatus,
        limit: Option<i64>,
    ) -> CustomResult<Vec<storage::ProcessTracker>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::ProcessTracker::find_processes_by_time_status(
            &conn,
            time_lower_limit,
            time_upper_limit,
            status,
            limit,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn insert_process(
        &self,
        new: storage::ProcessTrackerNew,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert_process(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn update_process(
        &self,
        this: storage::ProcessTracker,
        process: storage::ProcessTrackerUpdate,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        this.update(&conn, process)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    async fn reset_process(
        &self,
        this: storage::ProcessTracker,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<(), errors::StorageError> {
        self.update_process(
            this,
            storage::ProcessTrackerUpdate::StatusRetryUpdate {
                status: enums::ProcessTrackerStatus::New,
                retry_count: 0,
                schedule_time,
            },
        )
        .await?;
        Ok(())
    }

    async fn retry_process(
        &self,
        this: storage::ProcessTracker,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<(), errors::StorageError> {
        // TODO(jarnura): check how to handle metrics
        // metrics::TASK_RETRIED.add(1, &[]);
        let retry_count = this.retry_count + 1;
        self.update_process(
            this,
            storage::ProcessTrackerUpdate::StatusRetryUpdate {
                status: enums::ProcessTrackerStatus::Pending,
                retry_count,
                schedule_time,
            },
        )
        .await?;
        Ok(())
    }

    async fn finish_process_with_business_status(
        &self,
        this: storage::ProcessTracker,
        business_status: &'static str,
    ) -> CustomResult<(), errors::StorageError> {
        self.update_process(
            this,
            storage::ProcessTrackerUpdate::StatusUpdate {
                status: enums::ProcessTrackerStatus::Finish,
                business_status: Some(String::from(business_status)),
            },
        )
        .await
        .attach_printable("Failed to update business status of process")?;
        // TODO(jarnura): check how to handle metrics
        // metrics::TASK_FINISHED.add(1, &[]);
        Ok(())
    }

    async fn process_tracker_update_process_status_by_ids(
        &self,
        task_ids: Vec<String>,
        task_update: storage::ProcessTrackerUpdate,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::ProcessTracker::update_process_status_by_ids(&conn, task_ids, task_update)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}