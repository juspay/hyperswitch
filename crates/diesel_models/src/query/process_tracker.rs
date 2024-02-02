use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};
use router_env::{instrument, tracing};
use time::PrimitiveDateTime;

use super::generics;
use crate::{
    enums, errors,
    process_tracker::{
        ProcessTracker, ProcessTrackerNew, ProcessTrackerUpdate, ProcessTrackerUpdateInternal,
    },
    schema::process_tracker::dsl,
    PgPooledConn, StorageResult,
};

impl ProcessTrackerNew {
    #[instrument(skip(conn))]
        /// Inserts a new process into the database using the provided database connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the inserted `ProcessTracker` if successful, or an error if the insertion fails.
    pub async fn insert_process(self, conn: &PgPooledConn) -> StorageResult<ProcessTracker> {
        generics::generic_insert(conn, self).await
    }
}

impl ProcessTracker {
    #[instrument(skip(conn))]
        /// Asynchronously updates the process tracker with the provided data using the given database connection. If an error occurs during the update, the method handles the error and returns the updated process tracker if no fields were updated, or returns the error if there was a database error.
    pub async fn update(
        self,
        conn: &PgPooledConn,
        process: ProcessTrackerUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.id.clone(),
            ProcessTrackerUpdateInternal::from(process),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            result => result,
        }
    }

    #[instrument(skip(conn))]
        /// Asynchronously updates the process status for a list of task IDs in the database using the given process tracker update. It returns the number of rows affected by the update operation.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// * `task_ids` - A vector of task IDs for which the process status needs to be updated
    /// * `task_update` - The process tracker update containing the new status and other information
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the number of rows affected by the update operation
    pub async fn update_process_status_by_ids(
        conn: &PgPooledConn,
        task_ids: Vec<String>,
        task_update: ProcessTrackerUpdate,
    ) -> StorageResult<usize> {
        generics::generic_update::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq_any(task_ids),
            ProcessTrackerUpdateInternal::from(task_update),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a process by its ID in the database and returns it as an option.
    pub async fn find_process_by_id(conn: &PgPooledConn, id: &str) -> StorageResult<Option<Self>> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            id.to_owned(),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Queries the database to find processes within a specified time range and with a specific status,
    /// and returns them as a vector. An optional limit can be set to restrict the number of results.
    pub async fn find_processes_by_time_status(
        conn: &PgPooledConn,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        status: enums::ProcessTrackerStatus,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::schedule_time
                .between(time_lower_limit, time_upper_limit)
                .and(dsl::status.eq(status)),
            limit,
            None,
            None,
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds and returns a vector of processes to clean based on the given time limits, runner, and limit. 
    pub async fn find_processes_to_clean(
        conn: &PgPooledConn,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        runner: &str,
        limit: usize,
    ) -> StorageResult<Vec<Self>> {
        let mut x: Vec<Self> = generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::schedule_time
                .between(time_lower_limit, time_upper_limit)
                .and(dsl::status.eq(enums::ProcessTrackerStatus::ProcessStarted))
                .and(dsl::runner.eq(runner.to_owned())),
            None,
            None,
            None,
        )
        .await?;
        x.sort_by(|a, b| a.schedule_time.cmp(&b.schedule_time));
        x.truncate(limit);
        Ok(x)
    }

    #[instrument(skip(conn))]
        /// Asynchronously reinitializes limbo processes in the database by updating their status to 'Processing' and setting a new schedule time. The method takes a pooled PostgreSQL connection, a vector of process IDs, and a schedule time as input and returns a result containing the number of rows affected by the update operation.
    pub async fn reinitialize_limbo_processes(
        conn: &PgPooledConn,
        ids: Vec<String>,
        schedule_time: PrimitiveDateTime,
    ) -> StorageResult<usize> {
        generics::generic_update::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::status
                .eq(enums::ProcessTrackerStatus::ProcessStarted)
                .and(dsl::id.eq_any(ids)),
            (
                dsl::status.eq(enums::ProcessTrackerStatus::Processing),
                dsl::schedule_time.eq(schedule_time),
            ),
        )
        .await
    }
}
