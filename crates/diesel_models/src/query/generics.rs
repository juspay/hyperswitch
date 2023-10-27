use std::fmt::Debug;

use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable,
    debug_query,
    dsl::{Find, Limit},
    helper_types::{Filter, IntoBoxed},
    insertable::CanInsertInSingleQuery,
    pg::{Pg, PgConnection},
    query_builder::{
        AsChangeset, AsQuery, DeleteStatement, InsertStatement, IntoUpdateTarget, QueryFragment,
        QueryId, UpdateStatement,
    },
    query_dsl::{
        methods::{BoxedDsl, FilterDsl, FindDsl, LimitDsl, OffsetDsl, OrderDsl},
        LoadQuery, RunQueryDsl,
    },
    result::Error as DieselError,
    Expression, Insertable, QueryDsl, QuerySource, Table,
};
use error_stack::{report, IntoReport, ResultExt};
use router_env::{instrument, logger, tracing};

use crate::{
    errors::{self},
    PgPooledConn, StorageResult,
};

pub mod db_metrics {
    use router_env::opentelemetry::KeyValue;

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

        let attributes = [
            KeyValue::new("table", table_name.unwrap_or("undefined")),
            KeyValue::new("operation", format!("{:?}", operation)),
        ];

        crate::metrics::DATABASE_CALLS_COUNT.add(&crate::metrics::CONTEXT, 1, &attributes);
        crate::metrics::DATABASE_CALL_TIME.record(
            &crate::metrics::CONTEXT,
            time_elapsed.as_secs_f64(),
            &attributes,
        );

        output
    }
}

use db_metrics::*;

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_insert<T, V, R>(conn: &PgPooledConn, values: V) -> StorageResult<R>
where
    T: HasTable<Table = T> + Table + 'static + Debug,
    V: Debug + Insertable<T>,
    <T as QuerySource>::FromClause: QueryFragment<Pg> + Debug,
    <V as Insertable<T>>::Values: CanInsertInSingleQuery<Pg> + QueryFragment<Pg> + 'static,
    InsertStatement<T, <V as Insertable<T>>::Values>:
        AsQuery + LoadQuery<'static, PgConnection, R> + Send,
    R: Send + 'static,
{
    let debug_values = format!("{values:?}");

    let query = diesel::insert_into(<T as HasTable>::table()).values(values);
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    match track_database_call::<T, _, _>(query.get_result_async(conn), DatabaseOperation::Insert)
        .await
        .into_report()
    {
        Ok(value) => Ok(value),
        Err(err) => match err.current_context() {
            DieselError::DatabaseError(diesel::result::DatabaseErrorKind::UniqueViolation, _) => {
                Err(err).change_context(errors::DatabaseError::UniqueViolation)
            }
            _ => Err(err).change_context(errors::DatabaseError::Others),
        },
    }
    .attach_printable_lazy(|| format!("Error while inserting {debug_values}"))
}

#[instrument(level = "DEBUG", skip_all)]
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
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    track_database_call::<T, _, _>(query.execute_async(conn), DatabaseOperation::Update)
        .await
        .into_report()
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| format!("Error while updating {debug_values}"))
}

#[instrument(level = "DEBUG", skip_all)]
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
    R: Send + 'static,

    // For cloning query (UpdateStatement)
    <Filter<T, P> as HasTable>::Table: Clone,
    <Filter<T, P> as IntoUpdateTarget>::WhereClause: Clone,
    <V as AsChangeset>::Changeset: Clone,
    <<Filter<T, P> as HasTable>::Table as QuerySource>::FromClause: Clone,
{
    let debug_values = format!("{values:?}");

    let query = diesel::update(<T as HasTable>::table().filter(predicate)).set(values);

    match track_database_call::<T, _, _>(
        query.to_owned().get_results_async(conn),
        DatabaseOperation::UpdateWithResults,
    )
    .await
    {
        Ok(result) => {
            logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());
            Ok(result)
        }
        Err(DieselError::QueryBuilderError(_)) => {
            Err(report!(errors::DatabaseError::NoFieldsToUpdate))
                .attach_printable_lazy(|| format!("Error while updating {debug_values}"))
        }
        Err(DieselError::NotFound) => Err(report!(errors::DatabaseError::NotFound))
            .attach_printable_lazy(|| format!("Error while updating {debug_values}")),
        _ => Err(report!(errors::DatabaseError::Others))
            .attach_printable_lazy(|| format!("Error while updating {debug_values}")),
    }
}

#[instrument(level = "DEBUG", skip_all)]
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
    R: Send + 'static,

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
            .into_report()
            .attach_printable("Maybe not queried using a unique key")
        })?
}

#[instrument(level = "DEBUG", skip_all)]
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
    R: Send + 'static,
    Pk: Clone + Debug,

    // For cloning query (UpdateStatement)
    <Find<T, Pk> as HasTable>::Table: Clone,
    <Find<T, Pk> as IntoUpdateTarget>::WhereClause: Clone,
    <V as AsChangeset>::Changeset: Clone,
    <<Find<T, Pk> as HasTable>::Table as QuerySource>::FromClause: Clone,
{
    let debug_values = format!("{values:?}");

    let query = diesel::update(<T as HasTable>::table().find(id.to_owned())).set(values);

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
        Err(DieselError::QueryBuilderError(_)) => {
            Err(report!(errors::DatabaseError::NoFieldsToUpdate))
                .attach_printable_lazy(|| format!("Error while updating by ID {debug_values}"))
        }
        Err(DieselError::NotFound) => Err(report!(errors::DatabaseError::NotFound))
            .attach_printable_lazy(|| format!("Error while updating by ID {debug_values}")),
        _ => Err(report!(errors::DatabaseError::Others))
            .attach_printable_lazy(|| format!("Error while updating by ID {debug_values}")),
    }
}

#[instrument(level = "DEBUG", skip_all)]
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
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    track_database_call::<T, _, _>(query.execute_async(conn), DatabaseOperation::Delete)
        .await
        .into_report()
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| "Error while deleting")
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

#[instrument(level = "DEBUG", skip_all)]
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
    R: Send + Clone + 'static,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    track_database_call::<T, _, _>(
        query.get_results_async(conn),
        DatabaseOperation::DeleteWithResult,
    )
    .await
    .into_report()
    .change_context(errors::DatabaseError::Others)
    .attach_printable_lazy(|| "Error while deleting")
    .and_then(|result| {
        result.first().cloned().ok_or_else(|| {
            report!(errors::DatabaseError::NotFound)
                .attach_printable("Object to be deleted does not exist")
        })
    })
}

#[instrument(level = "DEBUG", skip_all)]
async fn generic_find_by_id_core<T, Pk, R>(conn: &PgPooledConn, id: Pk) -> StorageResult<R>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + RunQueryDsl<PgConnection> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'static, PgConnection, R>,
    Pk: Clone + Debug,
    R: Send + 'static,
{
    let query = <T as HasTable>::table().find(id.to_owned());
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    match track_database_call::<T, _, _>(query.first_async(conn), DatabaseOperation::FindOne)
        .await
        .into_report()
    {
        Ok(value) => Ok(value),
        Err(err) => match err.current_context() {
            DieselError::NotFound => Err(err).change_context(errors::DatabaseError::NotFound),
            _ => Err(err).change_context(errors::DatabaseError::Others),
        },
    }
    .attach_printable_lazy(|| format!("Error finding record by primary key: {id:?}"))
}

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_find_by_id<T, Pk, R>(conn: &PgPooledConn, id: Pk) -> StorageResult<R>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + RunQueryDsl<PgConnection> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'static, PgConnection, R>,
    Pk: Clone + Debug,
    R: Send + 'static,
{
    generic_find_by_id_core::<T, _, _>(conn, id).await
}

#[instrument(level = "DEBUG", skip_all)]
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
    R: Send + 'static,
{
    to_optional(generic_find_by_id_core::<T, _, _>(conn, id).await)
}

#[instrument(level = "DEBUG", skip_all)]
async fn generic_find_one_core<T, P, R>(conn: &PgPooledConn, predicate: P) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static,
{
    let query = <T as HasTable>::table().filter(predicate);
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    track_database_call::<T, _, _>(query.get_result_async(conn), DatabaseOperation::FindOne)
        .await
        .into_report()
        .map_err(|err| match err.current_context() {
            DieselError::NotFound => err.change_context(errors::DatabaseError::NotFound),
            _ => err.change_context(errors::DatabaseError::Others),
        })
        .attach_printable_lazy(|| "Error finding record by predicate")
}

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_find_one<T, P, R>(conn: &PgPooledConn, predicate: P) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static,
{
    generic_find_one_core::<T, _, _>(conn, predicate).await
}

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_find_one_optional<T, P, R>(
    conn: &PgPooledConn,
    predicate: P,
) -> StorageResult<Option<R>>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    Filter<T, P>: LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static,
{
    to_optional(generic_find_one_core::<T, _, _>(conn, predicate).await)
}

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_filter<T, P, O, R>(
    conn: &PgPooledConn,
    predicate: P,
    limit: Option<i64>,
    offset: Option<i64>,
    order: Option<O>,
) -> StorageResult<Vec<R>>
where
    T: HasTable<Table = T> + Table + BoxedDsl<'static, Pg> + 'static,
    IntoBoxed<'static, T, Pg>: FilterDsl<P, Output = IntoBoxed<'static, T, Pg>>
        + LimitDsl<Output = IntoBoxed<'static, T, Pg>>
        + OffsetDsl<Output = IntoBoxed<'static, T, Pg>>
        + OrderDsl<O, Output = IntoBoxed<'static, T, Pg>>
        + LoadQuery<'static, PgConnection, R>
        + QueryFragment<Pg>
        + Send,
    O: Expression,
    R: Send + 'static,
{
    let mut query = T::table().into_boxed();
    query = query.filter(predicate);

    if let Some(limit) = limit {
        query = query.limit(limit);
    }

    if let Some(offset) = offset {
        query = query.offset(offset);
    }

    if let Some(order) = order {
        query = query.order(order);
    }

    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    track_database_call::<T, _, _>(query.get_results_async(conn), DatabaseOperation::Filter)
        .await
        .into_report()
        .change_context(errors::DatabaseError::NotFound)
        .attach_printable_lazy(|| "Error filtering records by predicate")
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
