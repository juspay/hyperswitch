use common_utils::errors::CustomResult;
pub use diesel_models as storage;
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use storage_impl::{connection, errors, mock_db::MockDb};
use time::PrimitiveDateTime;

use crate::{metrics, scheduler::Store};

#[async_trait::async_trait]
pub trait ProcessTrackerInterface: Send + Sync + 'static {
    async fn reinitialize_limbo_processes(
        &self,
        ids: Vec<String>,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<usize, errors::StorageError>;

    async fn find_process_by_id(
        &self,
        id: &str,
    ) -> CustomResult<Option<storage::ProcessTracker>, errors::StorageError>;

    async fn update_process(
        &self,
        this: storage::ProcessTracker,
        process: storage::ProcessTrackerUpdate,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError>;

    async fn process_tracker_update_process_status_by_ids(
        &self,
        task_ids: Vec<String>,
        task_update: storage::ProcessTrackerUpdate,
    ) -> CustomResult<usize, errors::StorageError>;

    async fn insert_process(
        &self,
        new: storage::ProcessTrackerNew,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError>;

    async fn reset_process(
        &self,
        this: storage::ProcessTracker,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<(), errors::StorageError>;

    async fn retry_process(
        &self,
        this: storage::ProcessTracker,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<(), errors::StorageError>;

    async fn finish_process_with_business_status(
        &self,
        this: storage::ProcessTracker,
        business_status: String,
    ) -> CustomResult<(), errors::StorageError>;

    async fn find_processes_by_time_status(
        &self,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        status: storage_enums::ProcessTrackerStatus,
        limit: Option<i64>,
    ) -> CustomResult<Vec<storage::ProcessTracker>, errors::StorageError>;
}

#[async_trait::async_trait]
impl ProcessTrackerInterface for Store {
    async fn find_process_by_id(
        &self,
        id: &str,
    ) -> CustomResult<Option<storage::ProcessTracker>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::ProcessTracker::find_process_by_id(&conn, id)
            .await
            .map_err(Into::into)
    }

    async fn reinitialize_limbo_processes(
        &self,
        ids: Vec<String>,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::ProcessTracker::reinitialize_limbo_processes(&conn, ids, schedule_time)
            .await
            .map_err(Into::into)
    }

    async fn find_processes_by_time_status(
        &self,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        status: storage_enums::ProcessTrackerStatus,
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
        .map_err(Into::into)
    }

    async fn insert_process(
        &self,
        new: storage::ProcessTrackerNew,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert_process(&conn).await.map_err(Into::into)
    }

    async fn update_process(
        &self,
        this: storage::ProcessTracker,
        process: storage::ProcessTrackerUpdate,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        this.update(&conn, process).await.map_err(Into::into)
    }

    async fn reset_process(
        &self,
        this: storage::ProcessTracker,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<(), errors::StorageError> {
        self.update_process(
            this,
            storage::ProcessTrackerUpdate::StatusRetryUpdate {
                status: storage_enums::ProcessTrackerStatus::New,
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
        metrics::TASK_RETRIED.add(&metrics::CONTEXT, 1, &[]);
        let retry_count = this.retry_count + 1;
        self.update_process(
            this,
            storage::ProcessTrackerUpdate::StatusRetryUpdate {
                status: storage_enums::ProcessTrackerStatus::Pending,
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
        business_status: String,
    ) -> CustomResult<(), errors::StorageError> {
        self.update_process(
            this,
            storage::ProcessTrackerUpdate::StatusUpdate {
                status: storage_enums::ProcessTrackerStatus::Finish,
                business_status: Some(business_status),
            },
        )
        .await
        .attach_printable("Failed to update business status of process")?;
        metrics::TASK_FINISHED.add(&metrics::CONTEXT, 1, &[]);
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
            .map_err(Into::into)
    }
}

#[async_trait::async_trait]
impl ProcessTrackerInterface for MockDb {
    async fn find_process_by_id(
        &self,
        id: &str,
    ) -> CustomResult<Option<storage::ProcessTracker>, errors::StorageError> {
        let optional = self
            .processes
            .lock()
            .await
            .iter()
            .find(|process| process.id == id)
            .cloned();

        Ok(optional)
    }

    async fn reinitialize_limbo_processes(
        &self,
        _ids: Vec<String>,
        _schedule_time: PrimitiveDateTime,
    ) -> CustomResult<usize, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_processes_by_time_status(
        &self,
        _time_lower_limit: PrimitiveDateTime,
        _time_upper_limit: PrimitiveDateTime,
        _status: storage_enums::ProcessTrackerStatus,
        _limit: Option<i64>,
    ) -> CustomResult<Vec<storage::ProcessTracker>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn insert_process(
        &self,
        new: storage::ProcessTrackerNew,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        let mut processes = self.processes.lock().await;
        let process = storage::ProcessTracker {
            id: new.id,
            name: new.name,
            tag: new.tag,
            runner: new.runner,
            retry_count: new.retry_count,
            schedule_time: new.schedule_time,
            rule: new.rule,
            tracking_data: new.tracking_data,
            business_status: new.business_status,
            status: new.status,
            event: new.event,
            created_at: new.created_at,
            updated_at: new.updated_at,
        };
        processes.push(process.clone());
        Ok(process)
    }

    async fn update_process(
        &self,
        _this: storage::ProcessTracker,
        _process: storage::ProcessTrackerUpdate,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn reset_process(
        &self,
        _this: storage::ProcessTracker,
        _schedule_time: PrimitiveDateTime,
    ) -> CustomResult<(), errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn retry_process(
        &self,
        _this: storage::ProcessTracker,
        _schedule_time: PrimitiveDateTime,
    ) -> CustomResult<(), errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn finish_process_with_business_status(
        &self,
        _this: storage::ProcessTracker,
        _business_status: String,
    ) -> CustomResult<(), errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn process_tracker_update_process_status_by_ids(
        &self,
        _task_ids: Vec<String>,
        _task_update: storage::ProcessTrackerUpdate,
    ) -> CustomResult<usize, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
