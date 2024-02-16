use common_utils::errors::CustomResult;
pub use diesel_models as storage;
use diesel_models::enums as storage_enums;
use error_stack::{IntoReport, ResultExt};
use serde::Serialize;
use storage_impl::{connection, errors, mock_db::MockDb};
use time::PrimitiveDateTime;

use crate::{errors as sch_errors, metrics, scheduler::Store, SchedulerInterface};

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
    async fn update_process_tracker(
        &self,
        this: storage::ProcessTracker,
        process: storage::ProcessTrackerUpdate,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError>;

    async fn insert_process(
        &self,
        new: storage::ProcessTrackerNew,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError>;

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
            .into_report()
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
            .into_report()
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
        .into_report()
    }

    async fn insert_process(
        &self,
        new: storage::ProcessTrackerNew,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert_process(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_process(
        &self,
        this: storage::ProcessTracker,
        process: storage::ProcessTrackerUpdate,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        this.update(&conn, process)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_process_tracker(
        &self,
        this: storage::ProcessTracker,
        process: storage::ProcessTrackerUpdate,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        this.update(&conn, process)
            .await
            .map_err(Into::into)
            .into_report()
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
            .into_report()
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

    async fn update_process_tracker(
        &self,
        _this: storage::ProcessTracker,
        _process: storage::ProcessTrackerUpdate,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
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

#[async_trait::async_trait]
pub trait ProcessTrackerExt {
    fn is_valid_business_status(&self, valid_statuses: &[&str]) -> bool;

    fn make_process_tracker_new<T>(
        process_tracker_id: String,
        task: &str,
        runner: crate::types::ProcessTrackerRunner,
        tag: impl IntoIterator<Item = impl Into<String>>,
        tracking_data: T,
        schedule_time: PrimitiveDateTime,
    ) -> Result<storage::ProcessTrackerNew, sch_errors::ProcessTrackerError>
    where
        T: Serialize;

    async fn reset(
        self,
        db: &dyn SchedulerInterface,
        schedule_time: PrimitiveDateTime,
    ) -> Result<(), sch_errors::ProcessTrackerError>;

    async fn retry(
        self,
        db: &dyn SchedulerInterface,
        schedule_time: PrimitiveDateTime,
    ) -> Result<(), sch_errors::ProcessTrackerError>;

    async fn finish_with_status(
        self,
        db: &dyn SchedulerInterface,
        status: String,
    ) -> Result<(), sch_errors::ProcessTrackerError>;
}

#[async_trait::async_trait]
impl ProcessTrackerExt for storage::ProcessTracker {
    fn is_valid_business_status(&self, valid_statuses: &[&str]) -> bool {
        valid_statuses.iter().any(|x| x == &self.business_status)
    }

    fn make_process_tracker_new<T>(
        process_tracker_id: String,
        task: &str,
        runner: crate::types::ProcessTrackerRunner,
        tag: impl IntoIterator<Item = impl Into<String>>,
        tracking_data: T,
        schedule_time: PrimitiveDateTime,
    ) -> Result<storage::ProcessTrackerNew, sch_errors::ProcessTrackerError>
    where
        T: Serialize,
    {
        let current_time = common_utils::date_time::now();
        Ok(storage::ProcessTrackerNew {
            id: process_tracker_id,
            name: Some(String::from(task)),
            tag: tag.into_iter().map(Into::into).collect(),
            runner: Some(runner.to_string()),
            retry_count: 0,
            schedule_time: Some(schedule_time),
            rule: String::new(),
            tracking_data: serde_json::to_value(tracking_data)
                .map_err(|_| sch_errors::ProcessTrackerError::SerializationFailed)?,
            business_status: String::from("Pending"),
            status: storage_enums::ProcessTrackerStatus::New,
            event: vec![],
            created_at: current_time,
            updated_at: current_time,
        })
    }

    async fn reset(
        self,
        db: &dyn SchedulerInterface,
        schedule_time: PrimitiveDateTime,
    ) -> Result<(), sch_errors::ProcessTrackerError> {
        db.update_process_tracker(
            self.clone(),
            storage::ProcessTrackerUpdate::StatusRetryUpdate {
                status: storage_enums::ProcessTrackerStatus::New,
                retry_count: 0,
                schedule_time,
            },
        )
        .await?;
        Ok(())
    }

    async fn retry(
        self,
        db: &dyn SchedulerInterface,
        schedule_time: PrimitiveDateTime,
    ) -> Result<(), sch_errors::ProcessTrackerError> {
        metrics::TASK_RETRIED.add(&metrics::CONTEXT, 1, &[]);
        db.update_process_tracker(
            self.clone(),
            storage::ProcessTrackerUpdate::StatusRetryUpdate {
                status: storage_enums::ProcessTrackerStatus::Pending,
                retry_count: self.retry_count + 1,
                schedule_time,
            },
        )
        .await?;
        Ok(())
    }

    async fn finish_with_status(
        self,
        db: &dyn SchedulerInterface,
        status: String,
    ) -> Result<(), sch_errors::ProcessTrackerError> {
        db.update_process(
            self,
            storage::ProcessTrackerUpdate::StatusUpdate {
                status: storage_enums::ProcessTrackerStatus::Finish,
                business_status: Some(status),
            },
        )
        .await
        .attach_printable("Failed while updating status of the process")?;
        metrics::TASK_FINISHED.add(&metrics::CONTEXT, 1, &[]);
        Ok(())
    }
}
