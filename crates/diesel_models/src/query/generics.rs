use std::fmt::Debug;

use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable,
    debug_query,
    dsl::{count_star, Find, IsNotNull, Limit},
    helper_types::{Filter, IntoBoxed},
    insertable::CanInsertInSingleQuery,
    pg::Pg,
    query_builder::{
        AsChangeset, AsQuery, DeleteStatement, InsertStatement, IntoUpdateTarget, QueryFragment,
        QueryId, UpdateStatement,
    },
    query_dsl::{
        methods::{BoxedDsl, FilterDsl, FindDsl, LimitDsl, OffsetDsl, OrderDsl, SelectDsl},
        LoadQuery, RunQueryDsl,
    },
    result::Error as DieselError,
    Expression, ExpressionMethods, Insertable, QuerySource, Table,
};
use error_stack::{report, ResultExt};
#[cfg(feature = "deja")]
use hyperswitch_masking::PeekInterface;
use hyperswitch_masking::Secret;
use router_env::logger;

use crate::{
    errors, query::utils::GetPrimaryKey, DatabaseConnectionWithContext, DejaPgConnection,
    StorageResult,
};

pub mod db_metrics {
    use common_utils::external_service::{ExternalServiceCall, ExternalServiceEventEmitter};
    use diesel::result::Error as DieselError;

    #[derive(Debug)]
    pub enum DatabaseOperation {
        FindOne,
        Filter,
        Update,
        Insert,
        Delete,
        DeleteWithResult,
        UpdateWithResults,
        UpdateOne,
        Count,
    }

    pub trait DatabaseCallStatus {
        fn is_success(&self) -> bool;
    }

    impl<T> DatabaseCallStatus for Result<T, DieselError> {
        fn is_success(&self) -> bool {
            matches!(self, Ok(_) | Err(DieselError::NotFound))
        }
    }

    /// Row-returning executors resolve to the `(result, wire)` pair under `deja` (see
    /// `Captured` in this module); success is determined by the inner result alone.
    #[cfg(feature = "deja")]
    impl<T> DatabaseCallStatus for (Result<T, DieselError>, Option<Vec<deja::db::WireRow>>) {
        fn is_success(&self) -> bool {
            self.0.is_success()
        }
    }

    fn emit_database_call_event<U>(
        request_id: Option<&str>,
        event_emitter: &dyn ExternalServiceEventEmitter,
        table_name: &str,
        operation: &DatabaseOperation,
        time_elapsed: std::time::Duration,
        output: &U,
    ) where
        U: DatabaseCallStatus,
    {
        if let Some(request_id) = request_id {
            let success = output.is_success();
            event_emitter.emit_external_service_call(ExternalServiceCall {
                service_name: "database".to_string(),
                endpoint: table_name.to_string(),
                method: format!("{operation:?}"),
                request_id: request_id.to_string(),
                status_code: if success { 200 } else { 500 },
                success,
                latency_ms: time_elapsed.as_millis(),
                created_at_timestamp: common_utils::date_time::now()
                    .assume_utc()
                    .unix_timestamp_nanos(),
            });
        }
    }

    /// Times a single database call, records its latency metric, and emits one
    /// `ExternalServiceCall` event reflecting that call.
    ///
    /// When `request_id` is absent (background work, drainer, scheduler) no event is emitted: the
    /// correlator joins on `request_id` and cannot place request-less rows.
    #[inline]
    pub async fn track_database_call<T, Fut, U>(
        request_id: Option<&str>,
        event_emitter: &dyn ExternalServiceEventEmitter,
        operation: DatabaseOperation,
        future: Fut,
    ) -> U
    where
        Fut: std::future::Future<Output = U>,
        U: DatabaseCallStatus,
    {
        let start = std::time::Instant::now();
        let output = future.await;
        let time_elapsed = start.elapsed();

        let table_name = std::any::type_name::<T>().rsplit("::").nth(1);

        let attributes = router_env::metric_attributes!(
            ("table", table_name.unwrap_or("undefined")),
            ("operation", format!("{:?}", operation))
        );

        crate::metrics::DATABASE_CALLS_COUNT.add(1, attributes);
        crate::metrics::DATABASE_CALL_TIME.record(time_elapsed.as_secs_f64(), attributes);

        emit_database_call_event(
            request_id,
            event_emitter,
            table_name.unwrap_or("undefined"),
            &operation,
            time_elapsed,
            &output,
        );

        output
    }
}

use db_metrics::*;

fn table_name<T>() -> &'static str {
    std::any::type_name::<T>()
        .rsplit("::")
        .nth(1)
        .unwrap_or("unknown")
}

/// Bound alias for generic query results. Recording/replaying a db row needs
/// serde only when the `deja` feature is compiled in; default builds must not
/// force `Serialize`/`Deserialize` onto every row type (some rows — e.g.
/// `LockerMockUp` with raw card data — deliberately have no serde impls
/// outside deja builds).
#[cfg(feature = "deja")]
pub trait DejaQueryResult: Debug + serde::Serialize + serde::de::DeserializeOwned {}
#[cfg(feature = "deja")]
impl<T: Debug + serde::Serialize + serde::de::DeserializeOwned> DejaQueryResult for T {}
#[cfg(not(feature = "deja"))]
pub trait DejaQueryResult {}
#[cfg(not(feature = "deja"))]
impl<T> DejaQueryResult for T {}

// ---------------------------------------------------------------------------
// Deja boundary executors
// ---------------------------------------------------------------------------
// Each `generic_*` helper is split builder/executor: the PUBLIC builder keeps
// its exact pre-fold signature, constructs the diesel query plus its
// metrics-tracked future, and hands the capture-worthy values — `table`,
// `sql`, `inputs` — to a small PRIVATE executor whose real fn args ARE the
// boundary's args. The `#[deja::boundary]` attribute owns record/replay via
// the shared dispatch seam:
//   - `codec = ResultCodec<_, DatabaseError>` reconstructs the typed result on
//     substitution — a recording that threw replays the SAME `DatabaseError`
//     context ("recording threw ⇒ replay throws"). Row-returning executors
//     resolve to the `Captured` pair (below) and wrap the codec in
//     `WithWireCodec` — same tape envelope, `(value, None)` on substitution.
//   - `result = deja::db::recorded_output(..)` /
//     `recorded_output_with_wire(..)` is the explicit state-key / row-image /
//     binds-read-key producer (the recorder itself never infers). The `_with_wire`
//     form receives the physical wire rows IN-BAND from the pair the executor
//     returns; nothing ambient carries a capture to a boundary.
//   - `state_read`/`state_write`/`state_touch` declare the query-fingerprint
//     fallback key.
//   - `replay` is the per-op routing knob: writes and row-returning reads
//     `Execute` (run live against the per-correlation schema and
//     shadow-compare), the read-only scalar `count` `Substitute`s (a count is
//     a non-seedable scalar; re-running it against a partially-seeded schema
//     would only measure seed incompleteness).
// Args are `sql` plus `inputs.binds` as JSON; `debug_sql` only feeds read keys.
// All three are `Secret`-wrapped so `Debug` redacts them.
// Feature-off, every executor is a plain async fn passthrough.

/// Under `deja`, a row-returning query future resolves to the result paired
/// with the binary wire rows its own connection captured while producing it;
/// the pairing is lexical, riding the future's value to the boundary.
/// Feature-off the alias is the bare result.
#[cfg(feature = "deja")]
type Captured<T> = (T, Option<Vec<deja::db::WireRow>>);
#[cfg(not(feature = "deja"))]
type Captured<T> = T;

/// The statement and its binds as JSON, captured only while observation is active.
#[cfg(feature = "deja")]
fn capture<Q: QueryFragment<Pg>>(query: &Q) -> (String, serde_json::Value) {
    if deja::__private::observation_is_active() {
        capture_statement(query)
    } else {
        (String::new(), serde_json::Value::Null)
    }
}

/// What an observing call records.
#[cfg(feature = "deja")]
fn capture_statement<Q: QueryFragment<Pg>>(query: &Q) -> (String, serde_json::Value) {
    let captured = deja::db::capture_query(query);
    (captured.sql, captured.binds)
}
#[cfg(not(feature = "deja"))]
fn capture<Q>(_query: &Q) -> (String, serde_json::Value) {
    (String::new(), serde_json::Value::Null)
}

/// A unique violation writes no row, so replay serves the recorded error
/// instead of inserting into a store that lacks the conflicting row.
#[cfg(feature = "deja")]
fn is_unique_violation<R>(out: &Captured<StorageResult<R>>) -> bool {
    matches!(
        &out.0,
        Err(err) if matches!(err.current_context(), errors::DatabaseError::UniqueViolation)
    )
}

#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "db",
        component = "diesel_models::query::generics",
        operation = "generic_insert",
        op = Create,
        replay = Execute,
        effect = Db,
        returns = Value,
        codec = deja::codec::WithWireCodec::<deja::codec::ResultCodec<R, errors::DatabaseError>>,
        args = deja::db::args("generic_insert", table, sql.peek(), inputs.peek()),
        state_write = deja::db::query_state_key("generic_insert", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output_with_wire(deja::db::StateAxis::Write, table, debug_sql.peek(), &__deja_result.0, __deja_result.1.as_deref()),
        neutral_error = is_unique_violation::<R>,
    )
)]
async fn execute_generic_insert<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    debug_sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> Captured<StorageResult<R>>
where
    F: std::future::Future<Output = Captured<Result<R, DieselError>>> + Send,
    R: Send + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &debug_sql, &inputs);
    #[cfg(feature = "deja")]
    let (result, wire) = fut.await;
    #[cfg(not(feature = "deja"))]
    let result = fut.await;
    let mapped = match result {
        Ok(value) => Ok(value),
        Err(err) => match err {
            DieselError::DatabaseError(diesel::result::DatabaseErrorKind::UniqueViolation, _) => {
                Err(report!(err)).change_context(errors::DatabaseError::UniqueViolation)
            }
            _ => Err(report!(err)).change_context(errors::DatabaseError::Others),
        },
    };
    #[cfg(feature = "deja")]
    let mapped = (mapped, wire);
    mapped
}

#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "db",
        component = "diesel_models::query::generics",
        operation = "generic_update",
        op = Update,
        replay = Execute,
        effect = Db,
        returns = Count,
        codec = deja::codec::ResultCodec::<usize, errors::DatabaseError>,
        args = deja::db::args("generic_update", table, sql.peek(), inputs.peek()),
        state_touch = deja::db::query_state_key("generic_update", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output(deja::db::StateAxis::Touch, table, debug_sql.peek(), __deja_result),
    )
)]
async fn execute_generic_update<F>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    debug_sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<usize>
where
    F: std::future::Future<Output = Result<usize, DieselError>> + Send,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &debug_sql, &inputs);
    fut.await.change_context(errors::DatabaseError::Others)
}

#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "db",
        component = "diesel_models::query::generics",
        operation = "generic_update_with_results",
        op = Update,
        replay = Execute,
        effect = Db,
        returns = Rows,
        codec = deja::codec::WithWireCodec::<deja::codec::ResultCodec<Vec<R>, errors::DatabaseError>>,
        args = deja::db::args("generic_update_with_results", table, sql.peek(), inputs.peek()),
        state_touch = deja::db::query_state_key("generic_update_with_results", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output_with_wire(deja::db::StateAxis::Touch, table, debug_sql.peek(), &__deja_result.0, __deja_result.1.as_deref()),
    )
)]
async fn execute_generic_update_with_results<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    debug_sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> Captured<StorageResult<Vec<R>>>
where
    F: std::future::Future<Output = Captured<Result<Vec<R>, DieselError>>> + Send,
    R: Send + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &debug_sql, &inputs);
    #[cfg(feature = "deja")]
    let (result, wire) = fut.await;
    #[cfg(not(feature = "deja"))]
    let result = fut.await;
    let mapped = match result {
        Ok(result) => Ok(result),
        Err(DieselError::QueryBuilderError(_)) => {
            Err(report!(errors::DatabaseError::NoFieldsToUpdate))
        }
        Err(DieselError::NotFound) => Err(report!(errors::DatabaseError::NotFound)),
        Err(error) => Err(error).change_context(errors::DatabaseError::Others),
    };
    #[cfg(feature = "deja")]
    let mapped = (mapped, wire);
    mapped
}

#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "db",
        component = "diesel_models::query::generics",
        operation = "generic_update_by_id",
        op = Update,
        replay = Execute,
        effect = Db,
        returns = Value,
        codec = deja::codec::WithWireCodec::<deja::codec::ResultCodec<R, errors::DatabaseError>>,
        args = deja::db::args("generic_update_by_id", table, sql.peek(), inputs.peek()),
        state_touch = deja::db::query_state_key("generic_update_by_id", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output_with_wire(deja::db::StateAxis::Touch, table, debug_sql.peek(), &__deja_result.0, __deja_result.1.as_deref()),
    )
)]
async fn execute_generic_update_by_id<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    debug_sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> Captured<StorageResult<R>>
where
    F: std::future::Future<Output = Captured<Result<R, DieselError>>> + Send,
    R: Send + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &debug_sql, &inputs);
    #[cfg(feature = "deja")]
    let (result, wire) = fut.await;
    #[cfg(not(feature = "deja"))]
    let result = fut.await;
    let mapped = match result {
        Ok(result) => Ok(result),
        Err(DieselError::QueryBuilderError(_)) => {
            Err(report!(errors::DatabaseError::NoFieldsToUpdate))
        }
        Err(DieselError::NotFound) => Err(report!(errors::DatabaseError::NotFound)),
        Err(error) => Err(error).change_context(errors::DatabaseError::Others),
    };
    #[cfg(feature = "deja")]
    let mapped = (mapped, wire);
    mapped
}

#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "db",
        component = "diesel_models::query::generics",
        operation = "generic_delete",
        op = Delete,
        replay = Execute,
        effect = Db,
        returns = Bool,
        codec = deja::codec::ResultCodec::<bool, errors::DatabaseError>,
        args = deja::db::args("generic_delete", table, sql.peek(), inputs.peek()),
        state_touch = deja::db::query_state_key("generic_delete", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output(deja::db::StateAxis::Touch, table, debug_sql.peek(), __deja_result),
    )
)]
async fn execute_generic_delete<F>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    debug_sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<bool>
where
    F: std::future::Future<Output = Result<usize, DieselError>> + Send,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &debug_sql, &inputs);
    fut.await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while deleting")
        .and_then(|result| match result {
            n if n > 0 => {
                logger::debug!("{n} records deleted");
                Ok(true)
            }
            0 => {
                Err(report!(errors::DatabaseError::NotFound).attach_printable("No records deleted"))
            }
            _ => Ok(true), // n is usize, rustc requires this for exhaustive check
        })
}

#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "db",
        component = "diesel_models::query::generics",
        operation = "generic_delete_one_with_result",
        op = Delete,
        replay = Execute,
        effect = Db,
        returns = Value,
        codec = deja::codec::WithWireCodec::<deja::codec::ResultCodec<R, errors::DatabaseError>>,
        args = deja::db::args("generic_delete_one_with_result", table, sql.peek(), inputs.peek()),
        state_touch = deja::db::query_state_key("generic_delete_one_with_result", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output_with_wire(deja::db::StateAxis::Touch, table, debug_sql.peek(), &__deja_result.0, __deja_result.1.as_deref()),
    )
)]
async fn execute_generic_delete_one_with_result<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    debug_sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> Captured<StorageResult<R>>
where
    F: std::future::Future<Output = Captured<Result<Vec<R>, DieselError>>> + Send,
    R: Send + Clone + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &debug_sql, &inputs);
    #[cfg(feature = "deja")]
    let (result, wire) = fut.await;
    #[cfg(not(feature = "deja"))]
    let result = fut.await;
    let mapped = result
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while deleting")
        .and_then(|result| {
            result.first().cloned().ok_or_else(|| {
                report!(errors::DatabaseError::NotFound)
                    .attach_printable("Object to be deleted does not exist")
            })
        });
    #[cfg(feature = "deja")]
    let mapped = (mapped, wire);
    mapped
}

#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "db",
        component = "diesel_models::query::generics",
        operation = "generic_find_by_id_core",
        op = Read,
        replay = Execute,
        effect = Db,
        returns = Value,
        codec = deja::codec::WithWireCodec::<deja::codec::ResultCodec<R, errors::DatabaseError>>,
        args = deja::db::args("generic_find_by_id_core", table, sql.peek(), inputs.peek()),
        state_read = deja::db::query_state_key("generic_find_by_id_core", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output_with_wire(deja::db::StateAxis::Read, table, debug_sql.peek(), &__deja_result.0, __deja_result.1.as_deref()),
    )
)]
async fn execute_generic_find_by_id<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    debug_sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> Captured<StorageResult<R>>
where
    F: std::future::Future<Output = Captured<Result<R, DieselError>>> + Send,
    R: Send + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &debug_sql, &inputs);
    #[cfg(feature = "deja")]
    let (result, wire) = fut.await;
    #[cfg(not(feature = "deja"))]
    let result = fut.await;
    let mapped = match result {
        Ok(value) => Ok(value),
        Err(err) => match err {
            DieselError::NotFound => {
                Err(report!(err)).change_context(errors::DatabaseError::NotFound)
            }
            _ => Err(report!(err)).change_context(errors::DatabaseError::Others),
        },
    };
    #[cfg(feature = "deja")]
    let mapped = (mapped, wire);
    mapped
}

#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "db",
        component = "diesel_models::query::generics",
        operation = "generic_find_one_core",
        op = Read,
        replay = Execute,
        effect = Db,
        returns = Value,
        codec = deja::codec::WithWireCodec::<deja::codec::ResultCodec<R, errors::DatabaseError>>,
        args = deja::db::args("generic_find_one_core", table, sql.peek(), inputs.peek()),
        state_read = deja::db::query_state_key("generic_find_one_core", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output_with_wire(deja::db::StateAxis::Read, table, debug_sql.peek(), &__deja_result.0, __deja_result.1.as_deref()),
    )
)]
async fn execute_generic_find_one<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    debug_sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> Captured<StorageResult<R>>
where
    F: std::future::Future<Output = Captured<Result<R, DieselError>>> + Send,
    R: Send + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &debug_sql, &inputs);
    #[cfg(feature = "deja")]
    let (result, wire) = fut.await;
    #[cfg(not(feature = "deja"))]
    let result = fut.await;
    let mapped = result
        .map_err(|err| match err {
            DieselError::NotFound => report!(err).change_context(errors::DatabaseError::NotFound),
            _ => report!(err).change_context(errors::DatabaseError::Others),
        })
        .attach_printable("Error finding record by predicate");
    #[cfg(feature = "deja")]
    let mapped = (mapped, wire);
    mapped
}

#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "db",
        component = "diesel_models::query::generics",
        operation = "generic_filter",
        op = Read,
        replay = Execute,
        effect = Db,
        returns = Rows,
        codec = deja::codec::WithWireCodec::<deja::codec::ResultCodec<Vec<R>, errors::DatabaseError>>,
        args = deja::db::args("generic_filter", table, sql.peek(), inputs.peek()),
        state_read = deja::db::query_state_key("generic_filter", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output_with_wire(deja::db::StateAxis::Read, table, debug_sql.peek(), &__deja_result.0, __deja_result.1.as_deref()),
    )
)]
async fn execute_generic_filter<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    debug_sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> Captured<StorageResult<Vec<R>>>
where
    F: std::future::Future<Output = Captured<Result<Vec<R>, DieselError>>> + Send,
    R: Send + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &debug_sql, &inputs);
    #[cfg(feature = "deja")]
    let (result, wire) = fut.await;
    #[cfg(not(feature = "deja"))]
    let result = fut.await;
    let mapped = result
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error filtering records by predicate");
    #[cfg(feature = "deja")]
    let mapped = (mapped, wire);
    mapped
}

// Deja: no `on_miss`. A sync miss arm cannot run the count, and a made-up count
// would be treated as fact.
#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "db",
        component = "diesel_models::query::generics",
        operation = "generic_count",
        op = Read,
        replay = Substitute,
        effect = Db,
        returns = Count,
        codec = deja::codec::ResultCodec::<usize, errors::DatabaseError>,
        args = deja::db::args("generic_count", table, sql.peek(), inputs.peek()),
        state_read = deja::db::query_state_key("generic_count", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output(deja::db::StateAxis::Read, table, debug_sql.peek(), __deja_result),
    )
)]
async fn execute_generic_count<F>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    debug_sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<usize>
where
    F: std::future::Future<Output = Result<i64, DieselError>> + Send,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &debug_sql, &inputs);
    let count_i64: i64 = fut
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error counting records by predicate")?;

    let count_usize = usize::try_from(count_i64).map_err(|_| {
        report!(errors::DatabaseError::Others).attach_printable("Count value does not fit in usize")
    })?;

    Ok(count_usize)
}

// ---------------------------------------------------------------------------
// Public builders (signatures identical to pre-fold — zero call-site changes)
// ---------------------------------------------------------------------------

pub async fn generic_insert<T, V, R>(
    conn: &DatabaseConnectionWithContext<'_>,
    values: V,
) -> StorageResult<R>
where
    T: HasTable<Table = T> + Table + 'static + Debug,
    V: Debug + Insertable<T>,
    <T as QuerySource>::FromClause: QueryFragment<Pg> + Debug,
    <V as Insertable<T>>::Values: CanInsertInSingleQuery<Pg> + QueryFragment<Pg> + 'static,
    InsertStatement<T, <V as Insertable<T>>::Values>:
        AsQuery + LoadQuery<'static, DejaPgConnection, R> + Send,
    R: Send + 'static + DejaQueryResult,
{
    let debug_values = format!("{values:?}");

    let query = diesel::insert_into(<T as HasTable>::table()).values(values);
    let debug_sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %debug_sql);
    let (sql, binds) = capture(&query);
    let inputs = serde_json::json!({
        "binds": binds,
    });

    #[cfg(feature = "deja")]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::Insert,
        deja::db::get_result_captured(conn.raw_connection(), query),
    );
    #[cfg(not(feature = "deja"))]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::Insert,
        query.get_result_async(conn.raw_connection()),
    );

    let output = execute_generic_insert(
        fut,
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(debug_sql),
        Secret::new(inputs),
    )
    .await;
    #[cfg(feature = "deja")]
    let output = output.0;
    output.attach_printable_lazy(|| format!("Error while inserting {debug_values}"))
}

pub async fn generic_update<T, V, P>(
    conn: &DatabaseConnectionWithContext<'_>,
    predicate: P,
    values: V,
) -> StorageResult<usize>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    V: AsChangeset<Target = <Filter<T, P> as HasTable>::Table> + Debug,
    Filter<T, P>: IntoUpdateTarget,
    UpdateStatement<
        <Filter<T, P> as HasTable>::Table,
        <Filter<T, P> as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + QueryFragment<Pg> + QueryId + Send + 'static,
{
    let debug_values = format!("{values:?}");

    let query = diesel::update(<T as HasTable>::table().filter(predicate)).set(values);
    let debug_sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %debug_sql);
    let (sql, binds) = capture(&query);
    let inputs = serde_json::json!({
        "binds": binds,
        "predicate": { "type": std::any::type_name::<P>() },
    });

    execute_generic_update(
        track_database_call::<T, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            DatabaseOperation::Update,
            query.execute_async(conn.raw_connection()),
        ),
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(debug_sql),
        Secret::new(inputs),
    )
    .await
    .attach_printable_lazy(|| format!("Error while updating {debug_values}"))
}

pub async fn generic_update_with_results<T, V, P, R>(
    conn: &DatabaseConnectionWithContext<'_>,
    predicate: P,
    values: V,
) -> StorageResult<Vec<R>>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    V: AsChangeset<Target = <Filter<T, P> as HasTable>::Table> + Debug + 'static,
    Filter<T, P>: IntoUpdateTarget + 'static,
    UpdateStatement<
        <Filter<T, P> as HasTable>::Table,
        <Filter<T, P> as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + LoadQuery<'static, DejaPgConnection, R> + QueryFragment<Pg> + Send + Clone,
    R: Send + 'static + DejaQueryResult,

    // For cloning query (UpdateStatement)
    <Filter<T, P> as HasTable>::Table: Clone,
    <Filter<T, P> as IntoUpdateTarget>::WhereClause: Clone,
    <V as AsChangeset>::Changeset: Clone,
    <<Filter<T, P> as HasTable>::Table as QuerySource>::FromClause: Clone,
{
    let debug_values = format!("{values:?}");

    let query = diesel::update(<T as HasTable>::table().filter(predicate)).set(values);
    let debug_sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %debug_sql);
    let (sql, binds) = capture(&query);
    let inputs = serde_json::json!({
        "binds": binds,
        "predicate": { "type": std::any::type_name::<P>() },
    });

    #[cfg(feature = "deja")]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::UpdateWithResults,
        deja::db::get_results_captured(conn.raw_connection(), query.to_owned()),
    );
    #[cfg(not(feature = "deja"))]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::UpdateWithResults,
        query.to_owned().get_results_async(conn.raw_connection()),
    );

    let output = execute_generic_update_with_results(
        fut,
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(debug_sql),
        Secret::new(inputs),
    )
    .await;
    #[cfg(feature = "deja")]
    let output = output.0;
    output.attach_printable_lazy(|| format!("Error while updating {debug_values}"))
}

pub async fn generic_update_with_unique_predicate_get_result<T, V, P, R>(
    conn: &DatabaseConnectionWithContext<'_>,
    predicate: P,
    values: V,
) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    V: AsChangeset<Target = <Filter<T, P> as HasTable>::Table> + Debug + 'static,
    Filter<T, P>: IntoUpdateTarget + 'static,
    UpdateStatement<
        <Filter<T, P> as HasTable>::Table,
        <Filter<T, P> as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + LoadQuery<'static, DejaPgConnection, R> + QueryFragment<Pg> + Send,
    R: Send + 'static + DejaQueryResult,

    // For cloning query (UpdateStatement)
    <Filter<T, P> as HasTable>::Table: Clone,
    <Filter<T, P> as IntoUpdateTarget>::WhereClause: Clone,
    <V as AsChangeset>::Changeset: Clone,
    <<Filter<T, P> as HasTable>::Table as QuerySource>::FromClause: Clone,
{
    generic_update_with_results::<<T as HasTable>::Table, _, _, _>(conn, predicate, values)
        .await
        .map(|mut vec_r| {
            if vec_r.is_empty() {
                Err(errors::DatabaseError::NotFound)
            } else if vec_r.len() != 1 {
                Err(errors::DatabaseError::Others)
            } else {
                vec_r.pop().ok_or(errors::DatabaseError::Others)
            }
            .attach_printable("Maybe not queried using a unique key")
        })?
}

pub async fn generic_update_by_id<T, V, Pk, R>(
    conn: &DatabaseConnectionWithContext<'_>,
    id: Pk,
    values: V,
) -> StorageResult<R>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    V: AsChangeset<Target = <Find<T, Pk> as HasTable>::Table> + Debug,
    Find<T, Pk>:
        IntoUpdateTarget + QueryFragment<Pg> + RunQueryDsl<DejaPgConnection> + Send + 'static,
    UpdateStatement<
        <Find<T, Pk> as HasTable>::Table,
        <Find<T, Pk> as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + LoadQuery<'static, DejaPgConnection, R> + QueryFragment<Pg> + Send + 'static,
    Find<T, Pk>: LimitDsl,
    Limit<Find<T, Pk>>: LoadQuery<'static, DejaPgConnection, R>,
    R: Send + 'static + DejaQueryResult,
    Pk: Clone + Debug,

    // For cloning query (UpdateStatement)
    <Find<T, Pk> as HasTable>::Table: Clone,
    <Find<T, Pk> as IntoUpdateTarget>::WhereClause: Clone,
    <V as AsChangeset>::Changeset: Clone,
    <<Find<T, Pk> as HasTable>::Table as QuerySource>::FromClause: Clone,
{
    let debug_values = format!("{values:?}");

    let query = diesel::update(<T as HasTable>::table().find(id.to_owned())).set(values);
    let debug_sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %debug_sql);
    let (sql, binds) = capture(&query);
    let inputs = serde_json::json!({
        "binds": binds,
    });

    #[cfg(feature = "deja")]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::UpdateOne,
        deja::db::get_result_captured(conn.raw_connection(), query.to_owned()),
    );
    #[cfg(not(feature = "deja"))]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::UpdateOne,
        query.to_owned().get_result_async(conn.raw_connection()),
    );

    let output = execute_generic_update_by_id(
        fut,
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(debug_sql),
        Secret::new(inputs),
    )
    .await;
    #[cfg(feature = "deja")]
    let output = output.0;
    output.attach_printable_lazy(|| format!("Error while updating by ID {debug_values}"))
}

pub async fn generic_delete<T, P>(
    conn: &DatabaseConnectionWithContext<'_>,
    predicate: P,
) -> StorageResult<bool>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: IntoUpdateTarget,
    DeleteStatement<
        <Filter<T, P> as HasTable>::Table,
        <Filter<T, P> as IntoUpdateTarget>::WhereClause,
    >: AsQuery + QueryFragment<Pg> + QueryId + Send + 'static,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    let debug_sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %debug_sql);
    let (sql, binds) = capture(&query);
    let inputs = serde_json::json!({
        "binds": binds,
        "predicate": { "type": std::any::type_name::<P>() },
    });

    execute_generic_delete(
        track_database_call::<T, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            DatabaseOperation::Delete,
            query.execute_async(conn.raw_connection()),
        ),
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(debug_sql),
        Secret::new(inputs),
    )
    .await
}

pub async fn generic_delete_one_with_result<T, P, R>(
    conn: &DatabaseConnectionWithContext<'_>,
    predicate: P,
) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: IntoUpdateTarget,
    DeleteStatement<
        <Filter<T, P> as HasTable>::Table,
        <Filter<T, P> as IntoUpdateTarget>::WhereClause,
    >: AsQuery + LoadQuery<'static, DejaPgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + Clone + 'static + DejaQueryResult,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    let debug_sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %debug_sql);
    let (sql, binds) = capture(&query);
    let inputs = serde_json::json!({
        "binds": binds,
        "predicate": { "type": std::any::type_name::<P>() },
    });

    #[cfg(feature = "deja")]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::DeleteWithResult,
        deja::db::get_results_captured(conn.raw_connection(), query),
    );
    #[cfg(not(feature = "deja"))]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::DeleteWithResult,
        query.get_results_async(conn.raw_connection()),
    );

    let output = execute_generic_delete_one_with_result(
        fut,
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(debug_sql),
        Secret::new(inputs),
    )
    .await;
    #[cfg(feature = "deja")]
    let output = output.0;
    output
}

async fn generic_find_by_id_core<T, Pk, R>(
    conn: &DatabaseConnectionWithContext<'_>,
    id: Pk,
) -> StorageResult<R>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + RunQueryDsl<DejaPgConnection> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'static, DejaPgConnection, R>,
    Pk: Clone + Debug,
    R: Send + 'static + DejaQueryResult,
{
    let query = <T as HasTable>::table().find(id.to_owned());
    let debug_sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %debug_sql);
    let (sql, binds) = capture(&query);
    let inputs = serde_json::json!({
        "binds": binds,
    });

    #[cfg(feature = "deja")]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::FindOne,
        deja::db::first_captured(conn.raw_connection(), query),
    );
    #[cfg(not(feature = "deja"))]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::FindOne,
        query.first_async(conn.raw_connection()),
    );

    let output = execute_generic_find_by_id(
        fut,
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(debug_sql),
        Secret::new(inputs),
    )
    .await;
    #[cfg(feature = "deja")]
    let output = output.0;
    output.attach_printable_lazy(|| format!("Error finding record by primary key: {id:?}"))
}

pub async fn generic_find_by_id<T, Pk, R>(
    conn: &DatabaseConnectionWithContext<'_>,
    id: Pk,
) -> StorageResult<R>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + RunQueryDsl<DejaPgConnection> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'static, DejaPgConnection, R>,
    Pk: Clone + Debug,
    R: Send + 'static + DejaQueryResult,
{
    generic_find_by_id_core::<T, _, _>(conn, id).await
}

pub async fn generic_find_by_id_optional<T, Pk, R>(
    conn: &DatabaseConnectionWithContext<'_>,
    id: Pk,
) -> StorageResult<Option<R>>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    <T as HasTable>::Table: FindDsl<Pk>,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + RunQueryDsl<DejaPgConnection> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'static, DejaPgConnection, R>,
    Pk: Clone + Debug,
    R: Send + 'static + DejaQueryResult,
{
    to_optional(generic_find_by_id_core::<T, _, _>(conn, id).await)
}

async fn generic_find_one_core<T, P, R>(
    conn: &DatabaseConnectionWithContext<'_>,
    predicate: P,
) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: LoadQuery<'static, DejaPgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static + DejaQueryResult,
{
    let query = <T as HasTable>::table().filter(predicate);
    let debug_sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %debug_sql);
    let (sql, binds) = capture(&query);
    let inputs = serde_json::json!({
        "binds": binds,
        "predicate": { "type": std::any::type_name::<P>() },
    });

    #[cfg(feature = "deja")]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::FindOne,
        deja::db::get_result_captured(conn.raw_connection(), query),
    );
    #[cfg(not(feature = "deja"))]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::FindOne,
        query.get_result_async(conn.raw_connection()),
    );

    let output = execute_generic_find_one(
        fut,
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(debug_sql),
        Secret::new(inputs),
    )
    .await;
    #[cfg(feature = "deja")]
    let output = output.0;
    output
}

pub async fn generic_find_one<T, P, R>(
    conn: &DatabaseConnectionWithContext<'_>,
    predicate: P,
) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: LoadQuery<'static, DejaPgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static + DejaQueryResult,
{
    generic_find_one_core::<T, _, _>(conn, predicate).await
}

pub async fn generic_find_one_optional<T, P, R>(
    conn: &DatabaseConnectionWithContext<'_>,
    predicate: P,
) -> StorageResult<Option<R>>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: LoadQuery<'static, DejaPgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static + DejaQueryResult,
{
    to_optional(generic_find_one_core::<T, _, _>(conn, predicate).await)
}

pub(super) async fn generic_filter<T, P, O, R>(
    conn: &DatabaseConnectionWithContext<'_>,
    predicate: P,
    limit: Option<i64>,
    offset: Option<i64>,
    order: Option<O>,
) -> StorageResult<Vec<R>>
where
    T: HasTable<Table = T> + Table + BoxedDsl<'static, Pg> + GetPrimaryKey + 'static,
    IntoBoxed<'static, T, Pg>: FilterDsl<P, Output = IntoBoxed<'static, T, Pg>>
        + FilterDsl<IsNotNull<T::PK>, Output = IntoBoxed<'static, T, Pg>>
        + LimitDsl<Output = IntoBoxed<'static, T, Pg>>
        + OffsetDsl<Output = IntoBoxed<'static, T, Pg>>
        + OrderDsl<O, Output = IntoBoxed<'static, T, Pg>>
        + LoadQuery<'static, DejaPgConnection, R>
        + QueryFragment<Pg>
        + Send,
    O: Expression,
    R: Send + 'static + DejaQueryResult,
{
    let mut query = crate::list::into_boxed_list(T::table());
    query = query
        .filter(predicate)
        .filter(T::table().get_primary_key().is_not_null());
    if let Some(limit) = limit {
        query = query.limit(limit);
    }

    if let Some(offset) = offset {
        query = query.offset(offset);
    }

    if let Some(order) = order {
        query = query.order(order);
    }

    let debug_sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %debug_sql);
    let (sql, binds) = capture(&query);
    let inputs = serde_json::json!({
        "binds": binds,
        "predicate": { "type": std::any::type_name::<P>() },
        "limit": limit,
        "offset": offset,
        "order": { "type": std::any::type_name::<O>() },
    });

    #[cfg(feature = "deja")]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::Filter,
        deja::db::get_results_captured(conn.raw_connection(), query),
    );
    #[cfg(not(feature = "deja"))]
    let fut = track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        DatabaseOperation::Filter,
        query.get_results_async(conn.raw_connection()),
    );

    let output = execute_generic_filter(
        fut,
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(debug_sql),
        Secret::new(inputs),
    )
    .await;
    #[cfg(feature = "deja")]
    let output = output.0;
    output
}

pub async fn generic_count<T, P>(
    conn: &DatabaseConnectionWithContext<'_>,
    predicate: P,
) -> StorageResult<usize>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + SelectDsl<count_star> + 'static,
    Filter<T, P>: SelectDsl<count_star>,
    diesel::dsl::Select<Filter<T, P>, count_star>:
        LoadQuery<'static, DejaPgConnection, i64> + QueryFragment<Pg> + Send + 'static,
{
    let query = <T as HasTable>::table()
        .filter(predicate)
        .select(count_star());

    let debug_sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %debug_sql);
    let (sql, binds) = capture(&query);
    let inputs = serde_json::json!({
        "binds": binds,
        "predicate": { "type": std::any::type_name::<P>() },
    });

    execute_generic_count(
        track_database_call::<T, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            DatabaseOperation::Count,
            query.get_result_async(conn.raw_connection()),
        ),
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(debug_sql),
        Secret::new(inputs),
    )
    .await
}

fn to_optional<T>(arg: StorageResult<T>) -> StorageResult<Option<T>> {
    match arg {
        Ok(value) => Ok(Some(value)),
        Err(err) => match err.current_context() {
            errors::DatabaseError::NotFound => Ok(None),
            _ => Err(err),
        },
    }
}

#[cfg(all(test, feature = "deja", feature = "v1"))]
mod capture_tests {
    use diesel::{debug_query, pg::Pg, ExpressionMethods, QueryDsl};

    use super::{capture, capture_statement};
    use crate::schema::payment_attempt;

    /// A routing document with its keys inserted in `order`.
    fn routing(order: [&str; 2]) -> serde_json::Value {
        let mut inner = serde_json::Map::new();
        for method in order {
            inner.insert(
                method.to_owned(),
                serde_json::json!({ "connector": "adyen", "merchant_connector_id": "mca_1" }),
            );
        }
        serde_json::json!({ "algorithm": null, "pre_routing_results": inner })
    }

    fn update(value: serde_json::Value) -> impl diesel::query_builder::QueryFragment<Pg> {
        diesel::update(payment_attempt::table.filter(payment_attempt::attempt_id.eq("att_1")))
            .set(payment_attempt::straight_through_algorithm.eq(Some(value)))
    }

    /// The same map in two key orders captures identical args.
    #[test]
    fn one_routing_map_in_two_orders_records_the_same_args() {
        let (forward, backward) = (
            update(routing(["przelewy24", "ach"])),
            update(routing(["ach", "przelewy24"])),
        );
        assert_ne!(
            debug_query::<Pg, _>(&forward).to_string(),
            debug_query::<Pg, _>(&backward).to_string(),
            "premise: diesel renders the map in its iteration order"
        );
        let (left, right) = (capture_statement(&forward), capture_statement(&backward));
        assert_eq!(left, right);
        assert!(!left.0.contains("-- binds"), "{}", left.0);
        assert_eq!(
            left.1
                .pointer("/$1/pre_routing_results/ach/merchant_connector_id"),
            Some(&serde_json::json!("mca_1")),
            "the bound document is captured as a document: {}",
            left.1
        );
    }

    /// With nothing observing, `capture` returns the empty pair.
    #[test]
    fn an_unobserved_query_builds_no_statement() {
        let query = update(routing(["ach", "przelewy24"]));
        let idle = (String::new(), serde_json::Value::Null);
        assert_eq!(capture(&query), idle);
        assert_ne!(capture_statement(&query), idle);
    }
}

#[cfg(all(test, feature = "deja"))]
mod neutral_error_tests {
    use error_stack::report;

    use super::{errors, is_unique_violation, Captured, StorageResult};

    fn out(result: StorageResult<()>) -> Captured<StorageResult<()>> {
        (result, None)
    }

    /// Only a unique violation is state-neutral.
    #[test]
    fn only_a_unique_violation_is_state_neutral() {
        assert!(is_unique_violation(&out(Err(report!(
            errors::DatabaseError::UniqueViolation
        )))));
        assert!(!is_unique_violation(&out(Err(report!(
            errors::DatabaseError::NotFound
        )))));
        assert!(!is_unique_violation(&out(Ok(()))));
    }
}
