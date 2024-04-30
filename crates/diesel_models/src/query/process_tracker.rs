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
    //#\[instrument\(skip(conn))]
    pub async fn insert_process(self, conn: &PgPooledConn) -> StorageResult<ProcessTracker> {
        generics::generic_insert(conn, self).await
    }
}

impl ProcessTracker {
    //#\[instrument\(skip(conn))]
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

    //#\[instrument\(skip(conn))]
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

    //#\[instrument\(skip(conn))]
    pub async fn find_process_by_id(conn: &PgPooledConn, id: &str) -> StorageResult<Option<Self>> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            id.to_owned(),
        )
        .await
    }

    //#\[instrument\(skip(conn))]
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

    //#\[instrument\(skip(conn))]
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

    //#\[instrument\(skip(conn))]
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
