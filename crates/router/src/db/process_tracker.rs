use error_stack::IntoReport;
use time::PrimitiveDateTime;

use super::MockDb;
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage::{enums, ProcessTracker, ProcessTrackerNew, ProcessTrackerUpdate},
};

#[async_trait::async_trait]
pub trait ProcessTrackerInterface {
    async fn reinitialize_limbo_processes(
        &self,
        ids: Vec<String>,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<usize, errors::StorageError>;

    async fn find_process_by_id(
        &self,
        id: &str,
    ) -> CustomResult<Option<ProcessTracker>, errors::StorageError>;

    async fn update_process(
        &self,
        this: ProcessTracker,
        process: ProcessTrackerUpdate,
    ) -> CustomResult<ProcessTracker, errors::StorageError>;

    async fn process_tracker_update_process_status_by_ids(
        &self,
        task_ids: Vec<String>,
        task_update: ProcessTrackerUpdate,
    ) -> CustomResult<Vec<ProcessTracker>, errors::StorageError>;
    async fn update_process_tracker(
        &self,
        this: ProcessTracker,
        process: ProcessTrackerUpdate,
    ) -> CustomResult<ProcessTracker, errors::StorageError>;

    async fn insert_process(
        &self,
        new: ProcessTrackerNew,
    ) -> CustomResult<ProcessTracker, errors::StorageError>;

    async fn find_processes_by_time_status(
        &self,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        status: enums::ProcessTrackerStatus,
        limit: Option<i64>,
    ) -> CustomResult<Vec<ProcessTracker>, errors::StorageError>;
}

#[async_trait::async_trait]
impl ProcessTrackerInterface for super::Store {
    async fn find_process_by_id(
        &self,
        id: &str,
    ) -> CustomResult<Option<ProcessTracker>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        ProcessTracker::find_process_by_id(&conn, id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn reinitialize_limbo_processes(
        &self,
        ids: Vec<String>,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        ProcessTracker::reinitialize_limbo_processes(&conn, ids, schedule_time)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_processes_by_time_status(
        &self,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        status: enums::ProcessTrackerStatus,
        limit: Option<i64>,
    ) -> CustomResult<Vec<ProcessTracker>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        ProcessTracker::find_processes_by_time_status(
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
        new: ProcessTrackerNew,
    ) -> CustomResult<ProcessTracker, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        new.insert_process(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_process(
        &self,
        this: ProcessTracker,
        process: ProcessTrackerUpdate,
    ) -> CustomResult<ProcessTracker, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        this.update(&conn, process)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_process_tracker(
        &self,
        this: ProcessTracker,
        process: ProcessTrackerUpdate,
    ) -> CustomResult<ProcessTracker, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        this.update(&conn, process)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn process_tracker_update_process_status_by_ids(
        &self,
        task_ids: Vec<String>,
        task_update: ProcessTrackerUpdate,
    ) -> CustomResult<Vec<ProcessTracker>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        ProcessTracker::update_process_status_by_ids(&conn, task_ids, task_update)
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
    ) -> CustomResult<Option<ProcessTracker>, errors::StorageError> {
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
        todo!()
    }

    async fn find_processes_by_time_status(
        &self,
        _time_lower_limit: PrimitiveDateTime,
        _time_upper_limit: PrimitiveDateTime,
        _status: enums::ProcessTrackerStatus,
        _limit: Option<i64>,
    ) -> CustomResult<Vec<ProcessTracker>, errors::StorageError> {
        todo!()
    }

    async fn insert_process(
        &self,
        new: ProcessTrackerNew,
    ) -> CustomResult<ProcessTracker, errors::StorageError> {
        let mut processes = self.processes.lock().await;
        let process = ProcessTracker {
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
        _this: ProcessTracker,
        _process: ProcessTrackerUpdate,
    ) -> CustomResult<ProcessTracker, errors::StorageError> {
        todo!()
    }

    async fn update_process_tracker(
        &self,
        _this: ProcessTracker,
        _process: ProcessTrackerUpdate,
    ) -> CustomResult<ProcessTracker, errors::StorageError> {
        todo!()
    }

    async fn process_tracker_update_process_status_by_ids(
        &self,
        _task_ids: Vec<String>,
        _task_update: ProcessTrackerUpdate,
    ) -> CustomResult<Vec<ProcessTracker>, errors::StorageError> {
        todo!()
    }
}
