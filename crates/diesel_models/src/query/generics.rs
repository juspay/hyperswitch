use std::fmt::Debug;

use diesel_async::{methods::LoadQuery, RunQueryDsl};
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
    query_dsl::methods::{BoxedDsl, FilterDsl, FindDsl, LimitDsl, OffsetDsl, OrderDsl, SelectDsl},
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

pub fn generic_insert<'q, T, V, R>(
    conn: &'q mut PgPooledConn,
    values: V,
) -> impl std::future::Future<Output = StorageResult<R>> + Send + 'q
where
    T: HasTable<Table = T> + Table + 'static + Debug,
    V: Debug + Insertable<T> + 'q + Send,
    <T as QuerySource>::FromClause: QueryFragment<Pg> + Debug,
    <V as Insertable<T>>::Values: CanInsertInSingleQuery<Pg> + QueryFragment<Pg> + 'static,
    InsertStatement<T, <V as Insertable<T>>::Values>:
        AsQuery + LoadQuery<'q, PgPooledConn, R> + Send,
    R: Send + 'static,
{
    let debug_values = format!("{values:?}");

    let query = diesel::insert_into(<T as HasTable>::table()).values(values);
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    async move {
        match track_database_call::<T, _, _>(query.get_result(conn), DatabaseOperation::Insert)
            .await
        {
            Ok(value) => Ok(value),
            Err(err) => match err {
                DieselError::DatabaseError(diesel::result::DatabaseErrorKind::UniqueViolation, _) => {
                    Err(report!(err)).change_context(errors::DatabaseError::UniqueViolation)
                }
                _ => Err(report!(err)).change_context(errors::DatabaseError::Others),
            },
        }
        .attach_printable_lazy(|| format!("Error while inserting {debug_values}"))
    }
}

pub async fn generic_update<T, V, P>(
    conn: &mut PgPooledConn,
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

    track_database_call::<T, _, _>(query.execute(conn), DatabaseOperation::Update)
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| format!("Error while updating {debug_values}"))
}

pub fn generic_update_with_results<'q, T, V, P, R>(
    conn: &'q mut PgPooledConn,
    predicate: P,
    values: V,
) -> impl std::future::Future<Output = StorageResult<Vec<R>>> + Send + 'q
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    V: AsChangeset<Target = <Filter<T, P> as HasTable>::Table> + Debug + 'static,
    P: 'q + Send,
    Filter<T, P>: IntoUpdateTarget + 'static,
    UpdateStatement<
        <Filter<T, P> as HasTable>::Table,
        <Filter<T, P> as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + LoadQuery<'q, PgPooledConn, R> + QueryFragment<Pg> + Send + Clone,
    R: Send + 'static,

    // For cloning query (UpdateStatement)
    <Filter<T, P> as HasTable>::Table: Clone,
    <Filter<T, P> as IntoUpdateTarget>::WhereClause: Clone,
    <V as AsChangeset>::Changeset: Clone,
    <<Filter<T, P> as HasTable>::Table as QuerySource>::FromClause: Clone,
{
    let debug_values = format!("{values:?}");

    let query = diesel::update(<T as HasTable>::table().filter(predicate)).set(values);

    async move {
        match track_database_call::<T, _, _>(
            query.to_owned().get_results(conn),
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
            Err(error) => Err(error)
                .change_context(errors::DatabaseError::Others)
                .attach_printable_lazy(|| format!("Error while updating {debug_values}")),
        }
    }
}

pub fn generic_update_with_unique_predicate_get_result<'q, T, V, P, R>(
    conn: &'q mut PgPooledConn,
    predicate: P,
    values: V,
) -> impl std::future::Future<Output = StorageResult<R>> + Send + 'q
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    V: AsChangeset<Target = <Filter<T, P> as HasTable>::Table> + Debug + 'static + Send,
    P: 'q + Send,
    Filter<T, P>: IntoUpdateTarget + 'static,
    UpdateStatement<
        <Filter<T, P> as HasTable>::Table,
        <Filter<T, P> as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + LoadQuery<'q, PgPooledConn, R> + QueryFragment<Pg> + Send,
    R: Send + 'static,

    // For cloning query (UpdateStatement)
    <Filter<T, P> as HasTable>::Table: Clone,
    <Filter<T, P> as IntoUpdateTarget>::WhereClause: Clone,
    <V as AsChangeset>::Changeset: Clone,
    <<Filter<T, P> as HasTable>::Table as QuerySource>::FromClause: Clone,
{
    async move {
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
}

pub fn generic_update_by_id<'q, T, V, Pk, R>(
    conn: &'q mut PgPooledConn,
    id: Pk,
    values: V,
) -> impl std::future::Future<Output = StorageResult<R>> + Send + 'q
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    V: AsChangeset<Target = <Find<T, Pk> as HasTable>::Table> + Debug + 'q,
    Pk: Clone + Debug + 'q + Send,
    Find<T, Pk>: IntoUpdateTarget + QueryFragment<Pg> + Send + 'static,
    UpdateStatement<
        <Find<T, Pk> as HasTable>::Table,
        <Find<T, Pk> as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + LoadQuery<'q, PgPooledConn, R> + QueryFragment<Pg> + Send + 'static,
    Find<T, Pk>: LimitDsl,
    Limit<Find<T, Pk>>: LoadQuery<'q, PgPooledConn, R> + Send,
    R: Send + 'static,

    // For cloning query (UpdateStatement)
    <Find<T, Pk> as HasTable>::Table: Clone,
    <Find<T, Pk> as IntoUpdateTarget>::WhereClause: Clone,
    <V as AsChangeset>::Changeset: Clone,
    <<Find<T, Pk> as HasTable>::Table as QuerySource>::FromClause: Clone,
{
    let debug_values = format!("{values:?}");

    let query = diesel::update(<T as HasTable>::table().find(id.to_owned())).set(values);

    async move {
        match track_database_call::<T, _, _>(
            query.to_owned().get_result(conn),
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
            Err(error) => Err(error)
                .change_context(errors::DatabaseError::Others)
                .attach_printable_lazy(|| format!("Error while updating by ID {debug_values}")),
        }
    }
}

pub async fn generic_delete<T, P>(conn: &mut PgPooledConn, predicate: P) -> StorageResult<bool>
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

    track_database_call::<T, _, _>(query.execute(conn), DatabaseOperation::Delete)
        .await
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

pub fn generic_delete_one_with_result<'q, T, P, R>(
    conn: &'q mut PgPooledConn,
    predicate: P,
) -> impl std::future::Future<Output = StorageResult<R>> + Send + 'q
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    P: 'q + Send,
    Filter<T, P>: IntoUpdateTarget,
    DeleteStatement<
        <Filter<T, P> as HasTable>::Table,
        <Filter<T, P> as IntoUpdateTarget>::WhereClause,
    >: AsQuery + LoadQuery<'q, PgPooledConn, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + Clone + 'static,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    async move {
        track_database_call::<T, _, _>(
            query.get_results(conn),
            DatabaseOperation::DeleteWithResult,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while deleting")
        .and_then(|result| {
            result.get(0).cloned().ok_or_else(|| {
                report!(errors::DatabaseError::NotFound)
                    .attach_printable("Object to be deleted does not exist")
            })
        })
    }
}

pub fn generic_find_by_id_core<'q, T, Pk, R>(
    conn: &'q mut PgPooledConn,
    id: Pk,
) -> impl std::future::Future<Output = StorageResult<R>> + Send + 'q
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    Pk: Clone + Debug + 'q + Send,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'q, PgPooledConn, R> + Send,
    R: Send + 'static,
{
    let query = <T as HasTable>::table().find(id.to_owned());
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    async move {
        match track_database_call::<T, _, _>(query.first(conn), DatabaseOperation::FindOne).await
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
}

pub fn generic_find_by_id<'q, T, Pk, R>(
    conn: &'q mut PgPooledConn,
    id: Pk,
) -> impl std::future::Future<Output = StorageResult<R>> + Send + 'q
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    Pk: Clone + Debug + 'q + Send,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'q, PgPooledConn, R> + Send,
    R: Send + 'static,
{
    generic_find_by_id_core::<T, _, _>(conn, id)
}

pub fn generic_find_by_id_optional<'q, T, Pk, R>(
    conn: &'q mut PgPooledConn,
    id: Pk,
) -> impl std::future::Future<Output = StorageResult<Option<R>>> + Send + 'q
where
    T: FindDsl<Pk> + HasTable<Table = T> + LimitDsl + Table + 'static,
    <T as HasTable>::Table: FindDsl<Pk>,
    Pk: Clone + Debug + 'q + Send,
    Find<T, Pk>: LimitDsl + QueryFragment<Pg> + Send + 'static,
    Limit<Find<T, Pk>>: LoadQuery<'q, PgPooledConn, R> + Send,
    R: Send + 'static,
{
    async move { to_optional(generic_find_by_id_core::<T, _, _>(conn, id).await) }
}

pub fn generic_find_one_core<'q, T, P, R>(
    conn: &'q mut PgPooledConn,
    predicate: P,
) -> impl std::future::Future<Output = StorageResult<R>> + Send + 'q
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    P: 'q + Send,
    Filter<T, P>: LoadQuery<'q, PgPooledConn, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static,
{
    let query = <T as HasTable>::table().filter(predicate);
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    async move {
        track_database_call::<T, _, _>(query.get_result(conn), DatabaseOperation::FindOne)
            .await
            .map_err(|err| match err {
                DieselError::NotFound => report!(err).change_context(errors::DatabaseError::NotFound),
                _ => report!(err).change_context(errors::DatabaseError::Others),
            })
            .attach_printable("Error finding record by predicate")
    }
}

pub fn generic_find_one<'q, T, P, R>(
    conn: &'q mut PgPooledConn,
    predicate: P,
) -> impl std::future::Future<Output = StorageResult<R>> + Send + 'q
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    P: 'q + Send,
    Filter<T, P>: LoadQuery<'q, PgPooledConn, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static,
{
    generic_find_one_core::<T, _, _>(conn, predicate)
}

pub fn generic_find_one_optional<'q, T, P, R>(
    conn: &'q mut PgPooledConn,
    predicate: P,
) -> impl std::future::Future<Output = StorageResult<Option<R>>> + Send + 'q
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    P: 'q + Send,
    Filter<T, P>: LoadQuery<'q, PgPooledConn, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static,
{
    async move { to_optional(generic_find_one_core::<T, _, _>(conn, predicate).await) }
}

pub fn generic_filter<'q, T, P, O, R>(
    conn: &'q mut PgPooledConn,
    predicate: P,
    limit: Option<i64>,
    offset: Option<i64>,
    order: Option<O>,
) -> impl std::future::Future<Output = StorageResult<Vec<R>>> + Send + 'q
where
    T: HasTable<Table = T> + Table + BoxedDsl<'static, Pg> + GetPrimaryKey + 'static,
    P: 'q + Send,
    O: Expression + 'q + Send,
    IntoBoxed<'static, T, Pg>: FilterDsl<P, Output = IntoBoxed<'static, T, Pg>>
        + FilterDsl<IsNotNull<T::PK>, Output = IntoBoxed<'static, T, Pg>>
        + LimitDsl<Output = IntoBoxed<'static, T, Pg>>
        + OffsetDsl<Output = IntoBoxed<'static, T, Pg>>
        + OrderDsl<O, Output = IntoBoxed<'static, T, Pg>>
        + LoadQuery<'q, PgPooledConn, R>
        + QueryFragment<Pg>
        + Send,
    R: Send + 'static,
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

    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    async move {
        track_database_call::<T, _, _>(query.get_results(conn), DatabaseOperation::Filter)
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error filtering records by predicate")
    }
}

pub fn generic_count<'q, T, P>(
    conn: &'q mut PgPooledConn,
    predicate: P,
) -> impl std::future::Future<Output = StorageResult<usize>> + Send + 'q
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + SelectDsl<count_star> + 'static,
    P: 'q + Send,
    Filter<T, P>: SelectDsl<count_star>,
    diesel::dsl::Select<Filter<T, P>, count_star>:
        LoadQuery<'q, PgPooledConn, i64> + QueryFragment<Pg> + Send + 'static,
{
    let query = <T as HasTable>::table()
        .filter(predicate)
        .select(count_star());

    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    async move {
        let count_i64: i64 =
            track_database_call::<T, _, _>(query.get_result(conn), DatabaseOperation::Count)
                .await
                .change_context(errors::DatabaseError::Others)
                .attach_printable("Error counting records by predicate")?;

        let count_usize = usize::try_from(count_i64).map_err(|_| {
            report!(errors::DatabaseError::Others).attach_printable("Count value does not fit in usize")
        })?;

        Ok(count_usize)
    }
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
