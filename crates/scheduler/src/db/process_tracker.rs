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
        /// Asynchronously finds a process by its ID in the database. It queries the database to retrieve the process tracker with the specified ID, and returns it wrapped in an Option. If the process tracker is not found, it returns None. Any errors encountered during the database query are wrapped in a CustomResult with the specific StorageError type defined in the errors module.
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

        /// Asynchronously reinitializes limbo processes with the given ids and schedule time,
    /// returning the number of processes reinitialized or a StorageError if an error occurs.
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

        /// Retrieves a list of process tracker records from the database based on the specified time range and status.
    /// 
    /// # Arguments
    /// 
    /// * `time_lower_limit` - The lower limit of the time range for process creation.
    /// * `time_upper_limit` - The upper limit of the time range for process creation.
    /// * `status` - The status of the process tracker records to filter by.
    /// * `limit` - An optional limit to restrict the number of records returned.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of `storage::ProcessTracker` instances, or a `StorageError` if the operation fails.
    /// 
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

        /// Asynchronously inserts a new process into the storage system using the provided `ProcessTrackerNew` object. Returns a `CustomResult` containing a `ProcessTracker` if successful, or a `StorageError` if an error occurs.
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

        /// Asynchronously updates a process in the storage with the provided process tracker update.
    /// 
    /// # Arguments
    /// 
    /// * `this` - The process tracker to be updated.
    /// * `process` - The process tracker update containing the changes to be applied.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the updated `ProcessTracker` if successful, or a `StorageError` if an error occurs.
    ///
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

        /// Asynchronously updates the process tracker with the given process tracker update, and returns the updated process tracker. 
    /// 
    /// # Arguments
    /// * `this` - The current process tracker to be updated.
    /// * `process` - The process tracker update containing the changes to be applied.
    ///
    /// # Returns
    /// Returns a custom result containing the updated process tracker if the update is successful, otherwise returns a storage error. 
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

        /// Asynchronously processes the update of process status for a list of task IDs using the provided task update data.
    /// 
    /// # Arguments
    /// 
    /// * `task_ids` - A vector of strings representing the task IDs for which the process status is to be updated.
    /// * `task_update` - A `storage::ProcessTrackerUpdate` object containing the updated process status and other relevant data.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the number of task IDs for which the process status was updated, or a `StorageError` if an error occurs during the process.
    /// 
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
        /// Asynchronously finds a process by its ID in the storage. Returns a result containing an optional
    /// `storage::ProcessTracker` if the process is found, or an error of type `errors::StorageError` if
    /// any storage error occurs.
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

        /// Asynchronously reinitializes limbo processes for the given IDs at the specified schedule time.
    ///
    /// # Arguments
    ///
    /// * `ids` - A vector of strings representing the IDs of the processes to reinitialize.
    /// * `schedule_time` - A `PrimitiveDateTime` indicating the schedule time for reinitialization.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the number of processes reinitialized, or a `StorageError` if the reinitialization fails.
    ///
    async fn reinitialize_limbo_processes(
        &self,
        _ids: Vec<String>,
        _schedule_time: PrimitiveDateTime,
    ) -> CustomResult<usize, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously finds processes within a specified time range and status in the database.
    ///
    /// # Arguments
    ///
    /// * `time_lower_limit` - The lower limit of the time range.
    /// * `time_upper_limit` - The upper limit of the time range.
    /// * `status` - The status of the processes to be found.
    /// * `limit` - An optional limit on the number of processes to be returned.
    ///
    /// # Returns
    ///
    /// A vector of `ProcessTracker` objects within the specified time range and status, wrapped in a `CustomResult`. If an error occurs, a `StorageError` is returned.
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

        /// Inserts a new process into the process tracker and returns the inserted process.
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

        /// Asynchronously updates a process in the storage.
    ///
    /// # Arguments
    ///
    /// * `_this` - The current process tracker.
    /// * `_process` - The process tracker update to apply.
    ///
    /// # Returns
    ///
    /// * Returns a `CustomResult` containing the updated process tracker, or a `StorageError` if the update fails.
    ///
    async fn update_process(
        &self,
        _this: storage::ProcessTracker,
        _process: storage::ProcessTrackerUpdate,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously updates the process tracker with the given process tracker update.
    ///
    /// # Arguments
    ///
    /// * `_this` - The current process tracker to be updated.
    /// * `_process` - The update to be applied to the process tracker.
    ///
    /// # Returns
    ///
    /// The updated process tracker if successful, otherwise a `StorageError`.
    ///
    async fn update_process_tracker(
        &self,
        _this: storage::ProcessTracker,
        _process: storage::ProcessTrackerUpdate,
    ) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously processes the update of process status for the given task IDs using the provided ProcessTrackerUpdate.
    ///
    /// # Arguments
    ///
    /// * `_task_ids` - A vector of strings containing the task IDs for which the process status should be updated.
    /// * `_task_update` - A storage::ProcessTrackerUpdate struct containing the updated process status.
    ///
    /// # Returns
    ///
    /// A CustomResult containing the number of task updates processed or a StorageError if the operation fails.
    ///
    /// # Errors
    ///
    /// This method may return a StorageError::MockDbError if the operation encounters an error while interacting with the mock database.
    ///
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

    fn make_process_tracker_new<'a, T>(
        process_tracker_id: String,
        task: &'a str,
        runner: &'a str,
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
        /// Check if the business status of the instance is valid by comparing it with a list of valid statuses.
    /// 
    /// # Arguments
    /// 
    /// * `valid_statuses` - A reference to a slice of valid business statuses.
    /// 
    /// # Returns
    /// 
    /// * `bool` - Returns true if the business status of the instance matches any of the valid statuses, otherwise returns false.
    /// 
    fn is_valid_business_status(&self, valid_statuses: &[&str]) -> bool {
        valid_statuses.iter().any(|x| x == &self.business_status)
    }

        /// Creates a new process tracker with the given process tracker id, task, runner, tracking data, and schedule time.
    fn make_process_tracker_new<'a, T>(
        process_tracker_id: String,
        task: &'a str,
        runner: &'a str,
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
            tag: vec![String::from("SYNC"), String::from("PAYMENT")],
            runner: Some(String::from(runner)),
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

        /// Resets the process tracker in the database to a new status with a retry count of 0 and a new schedule time.
    ///
    /// # Arguments
    ///
    /// * `db` - A reference to a type implementing the SchedulerInterface trait, which is used for database operations.
    /// * `schedule_time` - The new schedule time for the process tracker.
    ///
    /// # Returns
    ///
    /// * `Result<(), sch_errors::ProcessTrackerError>` - A result indicating success or an error of type ProcessTrackerError.
    ///
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

        /// Retries the current task by updating the process tracker status to Pending and incrementing the retry count.
    ///
    /// # Arguments
    ///
    /// * `db` - A reference to a trait object implementing the SchedulerInterface.
    /// * `schedule_time` - The new schedule time for the retry.
    ///
    /// # Returns
    ///
    /// A Result indicating success or a ProcessTrackerError.
    ///
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
        /// Asynchronously updates the status of the current process in the database and increments a metric to indicate that a task has finished.
    ///
    /// # Arguments
    ///
    /// * `db` - A reference to a trait object implementing the `SchedulerInterface` trait for interacting with the database.
    /// * `status` - A `String` representing the new status to be set for the process.
    ///
    /// # Returns
    ///
    /// * `Result<(), sch_errors::ProcessTrackerError>` - A result indicating success or an error of type `ProcessTrackerError` if the update fails.
    ///
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
