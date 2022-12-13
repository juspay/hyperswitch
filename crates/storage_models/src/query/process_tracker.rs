use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, debug_query, pg::Pg, BoolExpressionMethods, ExpressionMethods, QueryDsl,
};
use error_stack::{IntoReport, ResultExt};
use router_env::{
    logger::debug,
    tracing::{self, instrument},
};
use time::PrimitiveDateTime;

use super::generics::{self, ExecuteQuery};
use crate::{
    enums, errors,
    process_tracker::{
        ProcessTracker, ProcessTrackerNew, ProcessTrackerUpdate, ProcessTrackerUpdateInternal,
    },
    schema::process_tracker::dsl,
    CustomResult, PgPooledConn,
};

impl ProcessTrackerNew {
    #[instrument(skip(conn))]
    pub async fn insert_process(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<ProcessTracker, errors::DatabaseError> {
        generics::generic_insert::<_, _, ProcessTracker, _>(conn, self, ExecuteQuery::new()).await
    }
}

impl ProcessTracker {
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        process: ProcessTrackerUpdate,
    ) -> CustomResult<Self, errors::DatabaseError> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            self.id.clone(),
            ProcessTrackerUpdateInternal::from(process),
            ExecuteQuery::new(),
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
    pub async fn update_process_status_by_ids(
        conn: &PgPooledConn,
        task_ids: Vec<String>,
        task_update: ProcessTrackerUpdate,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        // TODO: Possible optimization: Instead of returning updated values from database, update
        // the values in code and return them, if database query executed successfully.
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            dsl::id.eq_any(task_ids),
            ProcessTrackerUpdateInternal::from(task_update),
            ExecuteQuery::new(),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_process_by_id(
        conn: &PgPooledConn,
        id: &str,
    ) -> CustomResult<Option<Self>, errors::DatabaseError> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            id.to_owned(),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_processes_by_time_status(
        conn: &PgPooledConn,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        status: enums::ProcessTrackerStatus,
        limit: Option<i64>,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::schedule_time
                .between(time_lower_limit, time_upper_limit)
                .and(dsl::status.eq(status)),
            limit,
        )
        .await
    }

    // FIXME with generics
    #[instrument(skip(pool))]
    pub async fn find_processes_to_clean(
        pool: &PgPooledConn,
        time_lower_limit: PrimitiveDateTime,
        time_upper_limit: PrimitiveDateTime,
        runner: &str,
        limit: i64,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        let query = Self::table()
            .filter(dsl::schedule_time.between(time_lower_limit, time_upper_limit))
            .filter(dsl::status.eq(enums::ProcessTrackerStatus::ProcessStarted))
            .filter(dsl::runner.eq(runner.to_owned()))
            .order(dsl::schedule_time.asc())
            .limit(limit);
        debug!(query = %debug_query::<Pg, _>(&query).to_string());

        query
            .get_results_async(pool)
            .await
            .into_report()
            .change_context(errors::DatabaseError::NotFound)
            .attach_printable_lazy(|| "Error finding processes to clean")
    }

    #[instrument(skip(conn))]
    pub async fn reinitialize_limbo_processes(
        conn: &PgPooledConn,
        ids: Vec<String>,
        schedule_time: PrimitiveDateTime,
    ) -> CustomResult<usize, errors::DatabaseError> {
        generics::generic_update::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::status
                .eq(enums::ProcessTrackerStatus::ProcessStarted)
                .and(dsl::id.eq_any(ids)),
            (
                dsl::status.eq(enums::ProcessTrackerStatus::Processing),
                dsl::schedule_time.eq(schedule_time),
            ),
            ExecuteQuery::<Self>::new(),
        )
        .await
    }
}
