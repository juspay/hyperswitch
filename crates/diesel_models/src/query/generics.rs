use std::fmt::Debug;

use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable,
    debug_query,
    dsl::{count_star, Find, IsNotNull, Limit},
    helper_types::{Filter, IntoBoxed},
    insertable::CanInsertInSingleQuery,
    pg::{Pg, PgConnection},
    query_builder::{
        AsChangeset, AsQuery, DeleteStatement, InsertStatement, IntoUpdateTarget, QueryFragment,
        QueryId, UpdateStatement,
    },
    query_dsl::{
        methods::{BoxedDsl, FilterDsl, FindDsl, LimitDsl, OffsetDsl, OrderDsl, SelectDsl},
        LoadQuery, RunQueryDsl,
    },
    result::Error as DieselError,
    Expression, ExpressionMethods, Insertable, QueryDsl, QuerySource, Table,
};
use error_stack::{report, ResultExt};
#[cfg(feature = "deja")]
use hyperswitch_masking::PeekInterface;
use hyperswitch_masking::Secret;
use router_env::logger;

use crate::{errors, query::utils::GetPrimaryKey, PgPooledConn, StorageResult};

pub mod db_metrics {
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

    #[inline]
    pub async fn track_database_call<T, Fut, U>(future: Fut, operation: DatabaseOperation) -> U
    where
        Fut: std::future::Future<Output = U>,
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
//     context ("recording threw ⇒ replay throws").
//   - `result = deja::db::recorded_output(..)` is the explicit state-key /
//     row-image / binds-read-key producer (the recorder itself never infers).
//   - `state_read`/`state_write`/`state_touch` declare the query-fingerprint
//     fallback key.
//   - `replay` is the per-op routing knob: writes and row-returning reads
//     `Execute` (run live against the per-correlation schema and
//     shadow-compare), the read-only scalar `count` `Substitute`s (a count is
//     a non-seedable scalar; re-running it against a partially-seeded schema
//     would only measure seed incompleteness).
// `sql`/`inputs` are `Secret`-wrapped so their `Debug` output is redacted
// (bind values / changeset debug strings can carry PII); the deja attribute
// exprs `.peek()` them at the boundary, and the tape keeps full fidelity.
// Feature-off, every executor is a plain async fn passthrough.

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
        codec = deja::codec::ResultCodec::<R, errors::DatabaseError>,
        args = deja::db::args("generic_insert", table, sql.peek(), inputs.peek()),
        state_write = deja::db::query_state_key("generic_insert", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output(deja::db::StateAxis::Write, table, sql.peek(), __deja_result),
    )
)]
async fn execute_generic_insert<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<R>
where
    F: std::future::Future<Output = Result<R, DieselError>> + Send,
    R: Send + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &inputs);
    match fut.await {
        Ok(value) => Ok(value),
        Err(err) => match err {
            DieselError::DatabaseError(diesel::result::DatabaseErrorKind::UniqueViolation, _) => {
                Err(report!(err)).change_context(errors::DatabaseError::UniqueViolation)
            }
            _ => Err(report!(err)).change_context(errors::DatabaseError::Others),
        },
    }
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
        state_write = deja::db::query_state_key("generic_update", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output(deja::db::StateAxis::Write, table, sql.peek(), __deja_result),
    )
)]
async fn execute_generic_update<F>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<usize>
where
    F: std::future::Future<Output = Result<usize, DieselError>> + Send,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &inputs);
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
        codec = deja::codec::ResultCodec::<Vec<R>, errors::DatabaseError>,
        args = deja::db::args("generic_update_with_results", table, sql.peek(), inputs.peek()),
        state_touch = deja::db::query_state_key("generic_update_with_results", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output(deja::db::StateAxis::Touch, table, sql.peek(), __deja_result),
    )
)]
async fn execute_generic_update_with_results<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<Vec<R>>
where
    F: std::future::Future<Output = Result<Vec<R>, DieselError>> + Send,
    R: Send + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &inputs);
    match fut.await {
        Ok(result) => Ok(result),
        Err(DieselError::QueryBuilderError(_)) => {
            Err(report!(errors::DatabaseError::NoFieldsToUpdate))
        }
        Err(DieselError::NotFound) => Err(report!(errors::DatabaseError::NotFound)),
        Err(error) => Err(error).change_context(errors::DatabaseError::Others),
    }
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
        codec = deja::codec::ResultCodec::<R, errors::DatabaseError>,
        args = deja::db::args("generic_update_by_id", table, sql.peek(), inputs.peek()),
        state_touch = deja::db::query_state_key("generic_update_by_id", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output(deja::db::StateAxis::Touch, table, sql.peek(), __deja_result),
    )
)]
async fn execute_generic_update_by_id<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<R>
where
    F: std::future::Future<Output = Result<R, DieselError>> + Send,
    R: Send + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &inputs);
    match fut.await {
        Ok(result) => Ok(result),
        Err(DieselError::QueryBuilderError(_)) => {
            Err(report!(errors::DatabaseError::NoFieldsToUpdate))
        }
        Err(DieselError::NotFound) => Err(report!(errors::DatabaseError::NotFound)),
        Err(error) => Err(error).change_context(errors::DatabaseError::Others),
    }
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
        state_write = deja::db::query_state_key("generic_delete", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output(deja::db::StateAxis::Write, table, sql.peek(), __deja_result),
    )
)]
async fn execute_generic_delete<F>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<bool>
where
    F: std::future::Future<Output = Result<usize, DieselError>> + Send,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &inputs);
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
        codec = deja::codec::ResultCodec::<R, errors::DatabaseError>,
        args = deja::db::args("generic_delete_one_with_result", table, sql.peek(), inputs.peek()),
        state_touch = deja::db::query_state_key("generic_delete_one_with_result", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output(deja::db::StateAxis::Touch, table, sql.peek(), __deja_result),
    )
)]
async fn execute_generic_delete_one_with_result<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<R>
where
    F: std::future::Future<Output = Result<Vec<R>, DieselError>> + Send,
    R: Send + Clone + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &inputs);
    fut.await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while deleting")
        .and_then(|result| {
            result.first().cloned().ok_or_else(|| {
                report!(errors::DatabaseError::NotFound)
                    .attach_printable("Object to be deleted does not exist")
            })
        })
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
        codec = deja::codec::ResultCodec::<R, errors::DatabaseError>,
        args = deja::db::args("generic_find_by_id_core", table, sql.peek(), inputs.peek()),
        state_read = deja::db::query_state_key("generic_find_by_id_core", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output(deja::db::StateAxis::Read, table, sql.peek(), __deja_result),
    )
)]
async fn execute_generic_find_by_id<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<R>
where
    F: std::future::Future<Output = Result<R, DieselError>> + Send,
    R: Send + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &inputs);
    match fut.await {
        Ok(value) => Ok(value),
        Err(err) => match err {
            DieselError::NotFound => {
                Err(report!(err)).change_context(errors::DatabaseError::NotFound)
            }
            _ => Err(report!(err)).change_context(errors::DatabaseError::Others),
        },
    }
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
        codec = deja::codec::ResultCodec::<R, errors::DatabaseError>,
        args = deja::db::args("generic_find_one_core", table, sql.peek(), inputs.peek()),
        state_read = deja::db::query_state_key("generic_find_one_core", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output(deja::db::StateAxis::Read, table, sql.peek(), __deja_result),
    )
)]
async fn execute_generic_find_one<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<R>
where
    F: std::future::Future<Output = Result<R, DieselError>> + Send,
    R: Send + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &inputs);
    fut.await
        .map_err(|err| match err {
            DieselError::NotFound => report!(err).change_context(errors::DatabaseError::NotFound),
            _ => report!(err).change_context(errors::DatabaseError::Others),
        })
        .attach_printable("Error finding record by predicate")
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
        codec = deja::codec::ResultCodec::<Vec<R>, errors::DatabaseError>,
        args = deja::db::args("generic_filter", table, sql.peek(), inputs.peek()),
        state_read = deja::db::query_state_key("generic_filter", table, sql.peek(), inputs.peek()),
        result = deja::db::recorded_output(deja::db::StateAxis::Read, table, sql.peek(), __deja_result),
    )
)]
async fn execute_generic_filter<F, R>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<Vec<R>>
where
    F: std::future::Future<Output = Result<Vec<R>, DieselError>> + Send,
    R: Send + 'static + DejaQueryResult,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &inputs);
    fut.await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error filtering records by predicate")
}

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
        result = deja::db::recorded_output(deja::db::StateAxis::Read, table, sql.peek(), __deja_result),
    )
)]
async fn execute_generic_count<F>(
    fut: F,
    table: &'static str,
    sql: Secret<String>,
    inputs: Secret<serde_json::Value>,
) -> StorageResult<usize>
where
    F: std::future::Future<Output = Result<i64, DieselError>> + Send,
{
    #[cfg(not(feature = "deja"))]
    let _ = (&table, &sql, &inputs);
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

pub async fn generic_insert<T, V, R>(conn: &PgPooledConn, values: V) -> StorageResult<R>
where
    T: HasTable<Table = T> + Table + 'static + Debug,
    V: Debug + Insertable<T>,
    <T as QuerySource>::FromClause: QueryFragment<Pg> + Debug,
    <V as Insertable<T>>::Values: CanInsertInSingleQuery<Pg> + QueryFragment<Pg> + 'static,
    InsertStatement<T, <V as Insertable<T>>::Values>:
        AsQuery + LoadQuery<'static, PgConnection, R> + Send,
    R: Send + 'static + DejaQueryResult,
{
    let debug_values = format!("{values:?}");

    let query = diesel::insert_into(<T as HasTable>::table()).values(values);
    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let inputs = serde_json::json!({
        "values": { "debug": debug_values.as_str() },
    });

    execute_generic_insert(
        track_database_call::<T, _, _>(query.get_result_async(conn), DatabaseOperation::Insert),
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(inputs),
    )
    .await
    .attach_printable_lazy(|| format!("Error while inserting {debug_values}"))
}

pub async fn generic_update<T, V, P>(
    conn: &PgPooledConn,
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
    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let inputs = serde_json::json!({
        "values": { "debug": debug_values.as_str() },
        "predicate": { "type": std::any::type_name::<P>() },
    });

    execute_generic_update(
        track_database_call::<T, _, _>(query.execute_async(conn), DatabaseOperation::Update),
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(inputs),
    )
    .await
    .attach_printable_lazy(|| format!("Error while updating {debug_values}"))
}

pub async fn generic_update_with_results<T, V, P, R>(
    conn: &PgPooledConn,
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
    >: AsQuery + LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + Clone,
    R: Send + 'static + DejaQueryResult,

    // For cloning query (UpdateStatement)
    <Filter<T, P> as HasTable>::Table: Clone,
    <Filter<T, P> as IntoUpdateTarget>::WhereClause: Clone,
    <V as AsChangeset>::Changeset: Clone,
    <<Filter<T, P> as HasTable>::Table as QuerySource>::FromClause: Clone,
{
    let debug_values = format!("{values:?}");

    let query = diesel::update(<T as HasTable>::table().filter(predicate)).set(values);
    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let inputs = serde_json::json!({
        "values": { "debug": debug_values.as_str() },
        "predicate": { "type": std::any::type_name::<P>() },
    });

    execute_generic_update_with_results(
        track_database_call::<T, _, _>(
            query.to_owned().get_results_async(conn),
            DatabaseOperation::UpdateWithResults,
        ),
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(inputs),
    )
    .await
    .attach_printable_lazy(|| format!("Error while updating {debug_values}"))
}

pub async fn generic_update_with_unique_predicate_get_result<T, V, P, R>(
    conn: &PgPooledConn,
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
    >: AsQuery + LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send,
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
    conn: &PgPooledConn,
    id: Pk,
    values: V,
) -> StorageResult<R>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    V: AsChangeset<Target = <Find<T, Pk> as HasTable>::Table> + Debug,
    Find<T, Pk>: IntoUpdateTarget + QueryFragment<Pg> + RunQueryDsl<PgConnection> + Send + 'static,
    UpdateStatement<
        <Find<T, Pk> as HasTable>::Table,
        <Find<T, Pk> as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    Find<T, Pk>: LimitDsl,
    Limit<Find<T, Pk>>: LoadQuery<'static, PgConnection, R>,
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
    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let inputs = serde_json::json!({
        "id": { "debug": format!("{id:?}") },
        "values": { "debug": debug_values.as_str() },
    });

    execute_generic_update_by_id(
        track_database_call::<T, _, _>(
            query.to_owned().get_result_async(conn),
            DatabaseOperation::UpdateOne,
        ),
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(inputs),
    )
    .await
    .attach_printable_lazy(|| format!("Error while updating by ID {debug_values}"))
}

pub async fn generic_delete<T, P>(conn: &PgPooledConn, predicate: P) -> StorageResult<bool>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: IntoUpdateTarget,
    DeleteStatement<
        <Filter<T, P> as HasTable>::Table,
        <Filter<T, P> as IntoUpdateTarget>::WhereClause,
    >: AsQuery + QueryFragment<Pg> + QueryId + Send + 'static,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let inputs = serde_json::json!({
        "predicate": { "type": std::any::type_name::<P>() },
    });

    execute_generic_delete(
        track_database_call::<T, _, _>(query.execute_async(conn), DatabaseOperation::Delete),
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(inputs),
    )
    .await
}

pub async fn generic_delete_one_with_result<T, P, R>(
    conn: &PgPooledConn,
    predicate: P,
) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: IntoUpdateTarget,
    DeleteStatement<
        <Filter<T, P> as HasTable>::Table,
        <Filter<T, P> as IntoUpdateTarget>::WhereClause,
    >: AsQuery + LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + Clone + 'static + DejaQueryResult,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let inputs = serde_json::json!({
        "predicate": { "type": std::any::type_name::<P>() },
    });

    execute_generic_delete_one_with_result(
        track_database_call::<T, _, _>(
            query.get_results_async(conn),
            DatabaseOperation::DeleteWithResult,
        ),
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(inputs),
    )
    .await
}

async fn generic_find_by_id_core<T, Pk, R>(conn: &PgPooledConn, id: Pk) -> StorageResult<R>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + RunQueryDsl<PgConnection> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'static, PgConnection, R>,
    Pk: Clone + Debug,
    R: Send + 'static + DejaQueryResult,
{
    let query = <T as HasTable>::table().find(id.to_owned());
    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let inputs = serde_json::json!({
        "id": { "debug": format!("{id:?}") },
    });

    execute_generic_find_by_id(
        track_database_call::<T, _, _>(query.first_async(conn), DatabaseOperation::FindOne),
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(inputs),
    )
    .await
    .attach_printable_lazy(|| format!("Error finding record by primary key: {id:?}"))
}

pub async fn generic_find_by_id<T, Pk, R>(conn: &PgPooledConn, id: Pk) -> StorageResult<R>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + RunQueryDsl<PgConnection> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'static, PgConnection, R>,
    Pk: Clone + Debug,
    R: Send + 'static + DejaQueryResult,
{
    generic_find_by_id_core::<T, _, _>(conn, id).await
}

pub async fn generic_find_by_id_optional<T, Pk, R>(
    conn: &PgPooledConn,
    id: Pk,
) -> StorageResult<Option<R>>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    <T as HasTable>::Table: FindDsl<Pk>,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + RunQueryDsl<PgConnection> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'static, PgConnection, R>,
    Pk: Clone + Debug,
    R: Send + 'static + DejaQueryResult,
{
    to_optional(generic_find_by_id_core::<T, _, _>(conn, id).await)
}

async fn generic_find_one_core<T, P, R>(conn: &PgPooledConn, predicate: P) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static + DejaQueryResult,
{
    let query = <T as HasTable>::table().filter(predicate);
    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let inputs = serde_json::json!({
        "predicate": { "type": std::any::type_name::<P>() },
    });

    execute_generic_find_one(
        track_database_call::<T, _, _>(query.get_result_async(conn), DatabaseOperation::FindOne),
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(inputs),
    )
    .await
}

pub async fn generic_find_one<T, P, R>(conn: &PgPooledConn, predicate: P) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static + DejaQueryResult,
{
    generic_find_one_core::<T, _, _>(conn, predicate).await
}

pub async fn generic_find_one_optional<T, P, R>(
    conn: &PgPooledConn,
    predicate: P,
) -> StorageResult<Option<R>>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static + DejaQueryResult,
{
    to_optional(generic_find_one_core::<T, _, _>(conn, predicate).await)
}

pub(super) async fn generic_filter<T, P, O, R>(
    conn: &PgPooledConn,
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
        + LoadQuery<'static, PgConnection, R>
        + QueryFragment<Pg>
        + Send,
    O: Expression,
    R: Send + 'static + DejaQueryResult,
{
    let mut query = T::table().into_boxed();
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

    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let inputs = serde_json::json!({
        "predicate": { "type": std::any::type_name::<P>() },
        "limit": limit,
        "offset": offset,
        "order": { "type": std::any::type_name::<O>() },
    });

    execute_generic_filter(
        track_database_call::<T, _, _>(query.get_results_async(conn), DatabaseOperation::Filter),
        table_name::<T>(),
        Secret::new(sql),
        Secret::new(inputs),
    )
    .await
}

pub async fn generic_count<T, P>(conn: &PgPooledConn, predicate: P) -> StorageResult<usize>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + SelectDsl<count_star> + 'static,
    Filter<T, P>: SelectDsl<count_star>,
    diesel::dsl::Select<Filter<T, P>, count_star>:
        LoadQuery<'static, PgConnection, i64> + QueryFragment<Pg> + Send + 'static,
{
    let query = <T as HasTable>::table()
        .filter(predicate)
        .select(count_star());

    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let inputs = serde_json::json!({
        "predicate": { "type": std::any::type_name::<P>() },
    });

    execute_generic_count(
        track_database_call::<T, _, _>(query.get_result_async(conn), DatabaseOperation::Count),
        table_name::<T>(),
        Secret::new(sql),
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
