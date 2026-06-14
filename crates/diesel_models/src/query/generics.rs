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

#[cfg(feature = "deja")]
fn table_name<T>() -> &'static str {
    std::any::type_name::<T>()
        .rsplit("::")
        .nth(1)
        .unwrap_or("unknown")
}

#[cfg(feature = "deja")]
macro_rules! record_deja_db_query {
    ($operation:expr, $table:expr, $sql:expr, $inputs:expr, $kind:expr, $body:block) => {{
        deja::db::record_query_async(
            deja::db::QuerySpec::new($operation, $table, $sql, $inputs)
                .component("diesel_models::query::generics"),
            async move $body,
            $kind,
            // RECORD-side error-kind extraction. The macro knows the concrete
            // error is `Report<DatabaseError>`, so it matches `current_context()`
            // into a STABLE discriminant string (instead of recording a lossy
            // Debug blob). `message` keeps the human-readable text for diagnostics.
            |__deja_err: &error_stack::Report<errors::DatabaseError>|
                -> (::std::string::String, ::std::string::String) {
                let __kind = match __deja_err.current_context() {
                    errors::DatabaseError::NotFound => "NotFound",
                    errors::DatabaseError::UniqueViolation => "UniqueViolation",
                    _ => "Other",
                };
                (
                    __kind.to_string(),
                    format!("{__deja_err:?}"),
                )
            },
            // REPLAY-side faithful reconstruction of DETERMINISTIC control-flow
            // db errors, now via a STRUCTURED match on the recorded `kind` (not a
            // Debug-string scan). NotFound is branched on by "check-then-create"
            // logic (e.g. find_user_by_email → create); replaying it live would
            // hit the pg the record run mutated and invert the branch.
            // UniqueViolation likewise. Any other/unknown kind returns None and
            // falls through to live execution.
            |__deja_kind: &str, _msg: &str|
                -> ::core::option::Option<error_stack::Report<errors::DatabaseError>> {
                match __deja_kind {
                    "NotFound" => ::core::option::Option::Some(error_stack::report!(
                        errors::DatabaseError::NotFound
                    )),
                    "UniqueViolation" => ::core::option::Option::Some(error_stack::report!(
                        errors::DatabaseError::UniqueViolation
                    )),
                    _ => ::core::option::Option::None,
                }
            },
        )
        .await
    }};
}

#[cfg(not(feature = "deja"))]
macro_rules! record_deja_db_query {
    ($operation:expr, $table:expr, $sql:expr, $inputs:expr, $kind:expr, $body:block) => {{
        $body
    }};
}

/// Bound alias for generic query results. Recording/replaying a db row needs
/// serde only when the `deja` feature is compiled in; default builds must not
/// force `Serialize`/`Deserialize` onto every row type (some rows — e.g.
/// `LockerMockUp` with raw card data — deliberately have no serde impls
/// outside deja builds).
#[cfg(feature = "deja")]
pub trait DejaQueryResult: serde::Serialize + serde::de::DeserializeOwned {}
#[cfg(feature = "deja")]
impl<T: serde::Serialize + serde::de::DeserializeOwned> DejaQueryResult for T {}
#[cfg(not(feature = "deja"))]
pub trait DejaQueryResult {}
#[cfg(not(feature = "deja"))]
impl<T> DejaQueryResult for T {}

pub async fn generic_insert<T, V, R>(conn: &PgPooledConn, values: V) -> StorageResult<R>
where
    T: HasTable<Table = T> + Table + 'static + Debug,
    V: Debug + Insertable<T>,
    <T as QuerySource>::FromClause: QueryFragment<Pg> + Debug,
    <V as Insertable<T>>::Values: CanInsertInSingleQuery<Pg> + QueryFragment<Pg> + 'static,
    InsertStatement<T, <V as Insertable<T>>::Values>:
        AsQuery + LoadQuery<'static, PgConnection, R> + Send,
    R: Debug + Send + 'static + DejaQueryResult,
{
    let debug_values = format!("{values:?}");

    let query = diesel::insert_into(<T as HasTable>::table()).values(values);
    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let result = record_deja_db_query!(
        "generic_insert",
        table_name::<T>(),
        sql,
        serde_json::json!({
            "values": { "debug": debug_values.as_str() },
        }),
        deja::db::QueryResultKind::Value,
        {
            match track_database_call::<T, _, _>(
                query.get_result_async(conn),
                DatabaseOperation::Insert,
            )
            .await
            {
                Ok(value) => Ok(value),
                Err(err) => match err {
                    DieselError::DatabaseError(
                        diesel::result::DatabaseErrorKind::UniqueViolation,
                        _,
                    ) => Err(report!(err)).change_context(errors::DatabaseError::UniqueViolation),
                    _ => Err(report!(err)).change_context(errors::DatabaseError::Others),
                },
            }
            .attach_printable_lazy(|| format!("Error while inserting {debug_values}"))
        }
    );
    result
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
    let result = record_deja_db_query!(
        "generic_update",
        table_name::<T>(),
        sql,
        serde_json::json!({
            "values": { "debug": debug_values.as_str() },
            "predicate": { "type": std::any::type_name::<P>() },
        }),
        deja::db::QueryResultKind::Count,
        {
            track_database_call::<T, _, _>(query.execute_async(conn), DatabaseOperation::Update)
                .await
                .change_context(errors::DatabaseError::Others)
                .attach_printable_lazy(|| format!("Error while updating {debug_values}"))
        }
    );
    result
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
    R: Debug + Send + 'static + DejaQueryResult,

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
    let result = record_deja_db_query!(
        "generic_update_with_results",
        table_name::<T>(),
        sql,
        serde_json::json!({
            "values": { "debug": debug_values.as_str() },
            "predicate": { "type": std::any::type_name::<P>() },
        }),
        deja::db::QueryResultKind::Rows,
        {
            match track_database_call::<T, _, _>(
                query.to_owned().get_results_async(conn),
                DatabaseOperation::UpdateWithResults,
            )
            .await
            {
                Ok(result) => Ok(result),
                Err(DieselError::QueryBuilderError(_)) => {
                    Err(report!(errors::DatabaseError::NoFieldsToUpdate))
                        .attach_printable_lazy(|| format!("Error while updating {debug_values}"))
                }
                Err(DieselError::NotFound) => Err(report!(errors::DatabaseError::NotFound))
                    .attach_printable_lazy(|| format!("Error while updating {debug_values}")),
                Err(error) => Err(error)
                    .change_context(errors::DatabaseError::Others)
                    .attach_printable_lazy(|| format!("Error while updating {debug_values}")),
            }
        }
    );
    result
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
    R: Debug + Send + 'static + DejaQueryResult,

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
    R: Debug + Send + 'static + DejaQueryResult,
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
    let result = record_deja_db_query!(
        "generic_update_by_id",
        table_name::<T>(),
        sql,
        serde_json::json!({
            "id": { "debug": format!("{id:?}") },
            "values": { "debug": debug_values.as_str() },
        }),
        deja::db::QueryResultKind::Value,
        {
            match track_database_call::<T, _, _>(
                query.to_owned().get_result_async(conn),
                DatabaseOperation::UpdateOne,
            )
            .await
            {
                Ok(result) => {
                    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());
                    Ok(result)
                }
                Err(DieselError::QueryBuilderError(_)) => Err(report!(
                    errors::DatabaseError::NoFieldsToUpdate
                ))
                .attach_printable_lazy(|| format!("Error while updating by ID {debug_values}")),
                Err(DieselError::NotFound) => Err(report!(errors::DatabaseError::NotFound))
                    .attach_printable_lazy(|| format!("Error while updating by ID {debug_values}")),
                Err(error) => Err(error)
                    .change_context(errors::DatabaseError::Others)
                    .attach_printable_lazy(|| format!("Error while updating by ID {debug_values}")),
            }
        }
    );
    result
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
    let result = record_deja_db_query!(
        "generic_delete",
        table_name::<T>(),
        sql,
        serde_json::json!({
            "predicate": { "type": std::any::type_name::<P>() },
        }),
        deja::db::QueryResultKind::Bool,
        {
            track_database_call::<T, _, _>(query.execute_async(conn), DatabaseOperation::Delete)
                .await
                .change_context(errors::DatabaseError::Others)
                .attach_printable("Error while deleting")
                .and_then(|result| match result {
                    n if n > 0 => {
                        logger::debug!("{n} records deleted");
                        Ok(true)
                    }
                    0 => Err(report!(errors::DatabaseError::NotFound)
                        .attach_printable("No records deleted")),
                    _ => Ok(true), // n is usize, rustc requires this for exhaustive check
                })
        }
    );
    result
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
    R: Debug + Send + Clone + 'static + DejaQueryResult,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let result = record_deja_db_query!(
        "generic_delete_one_with_result",
        table_name::<T>(),
        sql,
        serde_json::json!({
            "predicate": { "type": std::any::type_name::<P>() },
        }),
        deja::db::QueryResultKind::Value,
        {
            track_database_call::<T, _, _>(
                query.get_results_async(conn),
                DatabaseOperation::DeleteWithResult,
            )
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error while deleting")
            .and_then(|result| {
                result.first().cloned().ok_or_else(|| {
                    report!(errors::DatabaseError::NotFound)
                        .attach_printable("Object to be deleted does not exist")
                })
            })
        }
    );
    result
}

async fn generic_find_by_id_core<T, Pk, R>(conn: &PgPooledConn, id: Pk) -> StorageResult<R>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + RunQueryDsl<PgConnection> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'static, PgConnection, R>,
    Pk: Clone + Debug,
    R: Debug + Send + 'static + DejaQueryResult,
{
    let query = <T as HasTable>::table().find(id.to_owned());
    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let result = record_deja_db_query!(
        "generic_find_by_id_core",
        table_name::<T>(),
        sql,
        serde_json::json!({
            "id": { "debug": format!("{id:?}") },
        }),
        deja::db::QueryResultKind::Value,
        {
            match track_database_call::<T, _, _>(
                query.first_async(conn),
                DatabaseOperation::FindOne,
            )
            .await
            {
                Ok(value) => Ok(value),
                Err(err) => match err {
                    DieselError::NotFound => {
                        Err(report!(err)).change_context(errors::DatabaseError::NotFound)
                    }
                    _ => Err(report!(err)).change_context(errors::DatabaseError::Others),
                },
            }
            .attach_printable_lazy(|| format!("Error finding record by primary key: {id:?}"))
        }
    );
    result
}

pub async fn generic_find_by_id<T, Pk, R>(conn: &PgPooledConn, id: Pk) -> StorageResult<R>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + RunQueryDsl<PgConnection> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'static, PgConnection, R>,
    Pk: Clone + Debug,
    R: Debug + Send + 'static + DejaQueryResult,
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
    R: Debug + Send + 'static + DejaQueryResult,
{
    to_optional(generic_find_by_id_core::<T, _, _>(conn, id).await)
}

async fn generic_find_one_core<T, P, R>(conn: &PgPooledConn, predicate: P) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Debug + Send + 'static + DejaQueryResult,
{
    let query = <T as HasTable>::table().filter(predicate);
    let sql = debug_query::<Pg, _>(&query).to_string();
    logger::debug!(query = %sql);
    let result = record_deja_db_query!(
        "generic_find_one_core",
        table_name::<T>(),
        sql,
        serde_json::json!({
            "predicate": { "type": std::any::type_name::<P>() },
        }),
        deja::db::QueryResultKind::Value,
        {
            track_database_call::<T, _, _>(query.get_result_async(conn), DatabaseOperation::FindOne)
                .await
                .map_err(|err| match err {
                    DieselError::NotFound => {
                        report!(err).change_context(errors::DatabaseError::NotFound)
                    }
                    _ => report!(err).change_context(errors::DatabaseError::Others),
                })
                .attach_printable("Error finding record by predicate")
        }
    );
    result
}

pub async fn generic_find_one<T, P, R>(conn: &PgPooledConn, predicate: P) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Debug + Send + 'static + DejaQueryResult,
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
    R: Debug + Send + 'static + DejaQueryResult,
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
    R: Debug + Send + 'static + DejaQueryResult,
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
    let result = record_deja_db_query!(
        "generic_filter",
        table_name::<T>(),
        sql,
        serde_json::json!({
            "predicate": { "type": std::any::type_name::<P>() },
            "limit": limit,
            "offset": offset,
            "order": { "type": std::any::type_name::<O>() },
        }),
        deja::db::QueryResultKind::Rows,
        {
            track_database_call::<T, _, _>(query.get_results_async(conn), DatabaseOperation::Filter)
                .await
                .change_context(errors::DatabaseError::Others)
                .attach_printable("Error filtering records by predicate")
        }
    );
    result
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
    let result = record_deja_db_query!(
        "generic_count",
        table_name::<T>(),
        sql,
        serde_json::json!({
            "predicate": { "type": std::any::type_name::<P>() },
        }),
        deja::db::QueryResultKind::Count,
        {
            let count_i64: i64 = track_database_call::<T, _, _>(
                query.get_result_async(conn),
                DatabaseOperation::Count,
            )
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error counting records by predicate")?;

            let count_usize = usize::try_from(count_i64).map_err(|_| {
                report!(errors::DatabaseError::Others)
                    .attach_printable("Count value does not fit in usize")
            })?;

            Ok(count_usize)
        }
    );
    result
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
