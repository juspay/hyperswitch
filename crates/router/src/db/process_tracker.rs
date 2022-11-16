use time::PrimitiveDateTime;

use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    services::Store,
    types::storage::{enums, ProcessTracker, ProcessTrackerNew, ProcessTrackerUpdate},
};

#[async_trait::async_trait]
pub trait IProcessTracker {
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
impl IProcessTracker for Store {
    async fn find_process_by_id(
        &self,
        id: &str,
    ) -> CustomResult<Option<ProcessTracker>, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        ProcessTracker::find_process_by_id(&conn, id).await
    }

    async fn reinitialize_limbo_processes(
        &self,
        ids: Vec<String>,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        ProcessTracker::reinitialize_limbo_processes(&conn, ids, schedule_time).await
    }

    async fn find_processes_by_time_status(
        &self,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        status: enums::ProcessTrackerStatus,
        limit: Option<i64>,
    ) -> CustomResult<Vec<ProcessTracker>, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        ProcessTracker::find_processes_by_time_status(
            &conn,
            time_lower_limit,
            time_upper_limit,
            status,
            limit,
        )
        .await
    }

    async fn insert_process(
        &self,
        new: ProcessTrackerNew,
    ) -> CustomResult<ProcessTracker, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        new.insert_process(&conn).await
    }

    async fn update_process(
        &self,
        this: ProcessTracker,
        process: ProcessTrackerUpdate,
    ) -> CustomResult<ProcessTracker, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        this.update(&conn, process).await
    }

    async fn update_process_tracker(
        &self,
        this: ProcessTracker,
        process: ProcessTrackerUpdate,
    ) -> CustomResult<ProcessTracker, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        this.update(&conn, process).await
    }

    async fn process_tracker_update_process_status_by_ids(
        &self,
        task_ids: Vec<String>,
        task_update: ProcessTrackerUpdate,
    ) -> CustomResult<Vec<ProcessTracker>, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        ProcessTracker::update_process_status_by_ids(&conn, task_ids, task_update).await
    }
}
