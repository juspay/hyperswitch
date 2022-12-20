use std::fmt::Debug;

use async_bb8_diesel::{AsyncRunQueryDsl, ConnectionError};
use diesel::{
    associations::HasTable,
    debug_query,
    dsl::{Find, Limit},
    insertable::CanInsertInSingleQuery,
    pg::{Pg, PgConnection},
    query_builder::{
        AsChangeset, AsQuery, DeleteStatement, InsertStatement, IntoUpdateTarget, QueryFragment,
        QueryId, UpdateStatement,
    },
    query_dsl::{
        methods::{FilterDsl, FindDsl, LimitDsl},
        LoadQuery, RunQueryDsl,
    },
    result::Error as DieselError,
    Insertable, QuerySource, Table,
};
use error_stack::{report, IntoReport, ResultExt};
use router_env::{logger, tracing, tracing::instrument};

use crate::{errors, PgPooledConn, StorageResult};

#[instrument(level = "DEBUG", skip_all)]
pub(super) async fn generic_insert<T, V, R>(conn: &PgPooledConn, values: V) -> StorageResult<R>
where
    T: HasTable<Table = T> + Table + 'static,
    V: Debug + Insertable<T>,
    <T as QuerySource>::FromClause: QueryFragment<Pg>,
    <V as Insertable<T>>::Values: CanInsertInSingleQuery<Pg> + QueryFragment<Pg> + 'static,
    InsertStatement<T, <V as Insertable<T>>::Values>:
        AsQuery + LoadQuery<'static, PgConnection, R> + Send,
    R: Send + 'static,
{
    let debug_values = format!("{:?}", values);

    let query = diesel::insert_into(<T as HasTable>::table()).values(values);
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    match query.get_result_async(conn).await.into_report() {
        Ok(value) => Ok(value),
        Err(err) => match err.current_context() {
            ConnectionError::Query(DieselError::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            )) => Err(err).change_context(errors::DatabaseError::UniqueViolation),
            _ => Err(err).change_context(errors::DatabaseError::Others),
        },
    }
    .attach_printable_lazy(|| format!("Error while inserting {}", debug_values))
}

#[instrument(level = "DEBUG", skip_all)]
pub(super) async fn generic_update<T, V, P>(
    conn: &PgPooledConn,
    predicate: P,
    values: V,
) -> StorageResult<usize>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    V: AsChangeset<Target = <<T as FilterDsl<P>>::Output as HasTable>::Table> + Debug,
    <T as FilterDsl<P>>::Output: IntoUpdateTarget,
    UpdateStatement<
        <<T as FilterDsl<P>>::Output as HasTable>::Table,
        <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + QueryFragment<Pg> + QueryId + Send + 'static,
{
    let debug_values = format!("{:?}", values);

    let query = diesel::update(<T as HasTable>::table().filter(predicate)).set(values);
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    query
        .execute_async(conn)
        .await
        .into_report()
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| format!("Error while updating {}", debug_values))
}

#[instrument(level = "DEBUG", skip_all)]
pub(super) async fn generic_update_with_results<T, V, P, R>(
    conn: &PgPooledConn,
    predicate: P,
    values: V,
) -> StorageResult<Vec<R>>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    V: AsChangeset<Target = <<T as FilterDsl<P>>::Output as HasTable>::Table> + Debug + 'static,
    <T as FilterDsl<P>>::Output: IntoUpdateTarget + 'static,
    UpdateStatement<
        <<T as FilterDsl<P>>::Output as HasTable>::Table,
        <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send,
    R: Send + 'static,
{
    let debug_values = format!("{:?}", values);

    let query = diesel::update(<T as HasTable>::table().filter(predicate)).set(values);
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    query
        .get_results_async(conn)
        .await
        .into_report()
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| format!("Error while updating {}", debug_values))
}

#[instrument(level = "DEBUG", skip_all)]
pub(super) async fn generic_update_by_id<T, V, Pk, R>(
    conn: &PgPooledConn,
    id: Pk,
    values: V,
) -> StorageResult<R>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    V: AsChangeset<Target = <<T as FindDsl<Pk>>::Output as HasTable>::Table> + Debug,
    <T as FindDsl<Pk>>::Output:
        IntoUpdateTarget + QueryFragment<Pg> + RunQueryDsl<PgConnection> + Send + 'static,
    UpdateStatement<
        <<T as FindDsl<Pk>>::Output as HasTable>::Table,
        <<T as FindDsl<Pk>>::Output as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    Find<T, Pk>: LimitDsl,
    Limit<Find<T, Pk>>: LoadQuery<'static, PgConnection, R>,
    R: Send + 'static,
    Pk: Clone + Debug,

    // For cloning query (UpdateStatement)
    <<T as FindDsl<Pk>>::Output as HasTable>::Table: Clone,
    <<T as FindDsl<Pk>>::Output as IntoUpdateTarget>::WhereClause: Clone,
    <V as AsChangeset>::Changeset: Clone,
    <<<T as FindDsl<Pk>>::Output as HasTable>::Table as QuerySource>::FromClause: Clone,
{
    let debug_values = format!("{:?}", values);

    let query = diesel::update(<T as HasTable>::table().find(id.to_owned())).set(values);

    match query.to_owned().get_result_async(conn).await {
        Ok(result) => {
            logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());
            Ok(result)
        }
        Err(ConnectionError::Query(DieselError::QueryBuilderError(_))) => {
            generic_find_by_id_core::<T, _, _>(conn, id).await
        }
        Err(ConnectionError::Query(DieselError::NotFound)) => {
            Err(report!(errors::DatabaseError::NotFound))
                .attach_printable_lazy(|| format!("Error while updating by ID {}", debug_values))
        }
        _ => Err(report!(errors::DatabaseError::Others))
            .attach_printable_lazy(|| format!("Error while updating by ID {}", debug_values)),
    }
}

#[instrument(level = "DEBUG", skip_all)]
pub(super) async fn generic_delete<T, P>(conn: &PgPooledConn, predicate: P) -> StorageResult<bool>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    <T as FilterDsl<P>>::Output: IntoUpdateTarget,
    DeleteStatement<
        <<T as FilterDsl<P>>::Output as HasTable>::Table,
        <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
    >: AsQuery + QueryFragment<Pg> + QueryId + Send + 'static,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    query
        .execute_async(conn)
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
pub(super) async fn generic_delete_one_with_result<T, P, R>(
    conn: &PgPooledConn,
    predicate: P,
) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    <T as FilterDsl<P>>::Output: IntoUpdateTarget,
    DeleteStatement<
        <<T as FilterDsl<P>>::Output as HasTable>::Table,
        <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
    >: AsQuery + LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + Clone + 'static,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    query
        .get_results_async(conn)
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
    <T as FindDsl<Pk>>::Output: QueryFragment<Pg> + RunQueryDsl<PgConnection> + Send + 'static,
    Find<T, Pk>: LimitDsl,
    Limit<Find<T, Pk>>: LoadQuery<'static, PgConnection, R>,
    Pk: Clone + Debug,
    R: Send + 'static,
{
    let query = <T as HasTable>::table().find(id.to_owned());
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    match query.first_async(conn).await.into_report() {
        Ok(value) => Ok(value),
        Err(err) => match err.current_context() {
            ConnectionError::Query(DieselError::NotFound) => {
                Err(err).change_context(errors::DatabaseError::NotFound)
            }
            _ => Err(err).change_context(errors::DatabaseError::Others),
        },
    }
    .attach_printable_lazy(|| format!("Error finding record by primary key: {:?}", id))
}

#[instrument(level = "DEBUG", skip_all)]
pub(super) async fn generic_find_by_id<T, Pk, R>(conn: &PgPooledConn, id: Pk) -> StorageResult<R>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    <T as FindDsl<Pk>>::Output: QueryFragment<Pg> + RunQueryDsl<PgConnection> + Send + 'static,
    Find<T, Pk>: LimitDsl,
    Limit<Find<T, Pk>>: LoadQuery<'static, PgConnection, R>,
    Pk: Clone + Debug,
    R: Send + 'static,
{
    generic_find_by_id_core::<T, _, _>(conn, id).await
}

#[instrument(level = "DEBUG", skip_all)]
pub(super) async fn generic_find_by_id_optional<T, Pk, R>(
    conn: &PgPooledConn,
    id: Pk,
) -> StorageResult<Option<R>>
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    <T as HasTable>::Table: FindDsl<Pk>,
    <<T as HasTable>::Table as FindDsl<Pk>>::Output: RunQueryDsl<PgConnection> + Send + 'static,
    <T as FindDsl<Pk>>::Output: QueryFragment<Pg>,
    Find<T, Pk>: LimitDsl,
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
    <T as FilterDsl<P>>::Output:
        LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static,
{
    let query = <T as HasTable>::table().filter(predicate);
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    query
        .get_result_async(conn)
        .await
        .into_report()
        .map_err(|err| match err.current_context() {
            ConnectionError::Query(DieselError::NotFound) => {
                err.change_context(errors::DatabaseError::NotFound)
            }
            _ => err.change_context(errors::DatabaseError::Others),
        })
        .attach_printable_lazy(|| "Error finding record by predicate")
}

#[instrument(level = "DEBUG", skip_all)]
pub(super) async fn generic_find_one<T, P, R>(conn: &PgPooledConn, predicate: P) -> StorageResult<R>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    <T as FilterDsl<P>>::Output:
        LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static,
{
    generic_find_one_core::<T, _, _>(conn, predicate).await
}

#[instrument(level = "DEBUG", skip_all)]
pub(super) async fn generic_find_one_optional<T, P, R>(
    conn: &PgPooledConn,
    predicate: P,
) -> StorageResult<Option<R>>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    <T as FilterDsl<P>>::Output:
        LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static,
{
    to_optional(generic_find_one_core::<T, _, _>(conn, predicate).await)
}

#[instrument(level = "DEBUG", skip_all)]
pub(super) async fn generic_filter<T, P, R>(
    conn: &PgPooledConn,
    predicate: P,
    limit: Option<i64>,
) -> StorageResult<Vec<R>>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    <T as FilterDsl<P>>::Output: LoadQuery<'static, PgConnection, R> + QueryFragment<Pg>,
    <T as FilterDsl<P>>::Output: LimitDsl + Send + 'static,
    <<T as FilterDsl<P>>::Output as LimitDsl>::Output:
        LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send,
    R: Send + 'static,
{
    let query = <T as HasTable>::table().filter(predicate);

    match limit {
        Some(limit) => {
            let query = query.limit(limit);
            logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());
            query.get_results_async(conn)
        }
        None => {
            logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());
            query.get_results_async(conn)
        }
    }
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
