use std::{fmt::Debug, marker::PhantomData};

use async_bb8_diesel::{AsyncRunQueryDsl, ConnectionError};
use async_trait::async_trait;
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

use crate::{errors, CustomResult, PgPooledConn};

#[derive(Debug)]
pub struct RawSqlQuery {
    pub sql: String,
    // The inner `Vec<u8>` can be considered to be byte array
    pub binds: Vec<Option<Vec<u8>>>,
}

impl RawSqlQuery {
    pub fn to_field_value_pairs(&self) -> Vec<(&str, String)> {
        vec![
            ("sql", self.sql.clone()),
            (
                "binds",
                serde_json::to_string(
                    &self
                        .binds
                        .iter()
                        .map(|bytes| bytes.as_ref().map(hex::encode))
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
            ),
        ]
    }
}

pub struct ExecuteQuery<R>(PhantomData<R>);

impl<R> ExecuteQuery<R> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<R> Default for ExecuteQuery<R> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RawQuery;

#[async_trait]
pub trait QueryExecutionMode<Q>
where
    Q: QueryFragment<Pg> + Send + 'static,
{
    type InsertOutput;
    type UpdateOutput;
    type UpdateWithResultsOutput;
    type UpdateByIdOutput;
    type DeleteOutput;
    type DeleteWithResultsOutput;
    type DeleteOneWithResultOutput;

    async fn insert(
        &self,
        conn: &PgPooledConn,
        query: Q,
        debug_values: String,
    ) -> CustomResult<Self::InsertOutput, errors::DatabaseError>
    where
        Q: AsQuery + QueryFragment<Pg> + RunQueryDsl<PgConnection>;

    async fn update(
        &self,
        conn: &PgPooledConn,
        query: Q,
        debug_values: String,
    ) -> CustomResult<Self::UpdateOutput, errors::DatabaseError>
    where
        Q: QueryId;

    async fn update_with_results(
        &self,
        conn: &PgPooledConn,
        query: Q,
        debug_values: String,
    ) -> CustomResult<Self::UpdateWithResultsOutput, errors::DatabaseError>;

    async fn update_by_id(
        &self,
        conn: &PgPooledConn,
        query: Q,
        debug_values: String,
    ) -> CustomResult<Self::UpdateByIdOutput, errors::DatabaseError>
    where
        Q: Clone;

    async fn delete(
        &self,
        conn: &PgPooledConn,
        query: Q,
    ) -> CustomResult<Self::DeleteOutput, errors::DatabaseError>
    where
        Q: QueryId;

    async fn delete_with_results(
        &self,
        conn: &PgPooledConn,
        query: Q,
    ) -> CustomResult<Self::DeleteWithResultsOutput, errors::DatabaseError>;

    async fn delete_one_with_result(
        &self,
        conn: &PgPooledConn,
        query: Q,
    ) -> CustomResult<Self::DeleteOneWithResultOutput, errors::DatabaseError>;
}

#[async_trait]
impl<Q, R> QueryExecutionMode<Q> for ExecuteQuery<R>
where
    Q: LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + Sync + 'static,
    R: Send + Sync + 'static,
{
    type InsertOutput = R;
    type UpdateOutput = usize;
    type UpdateWithResultsOutput = Vec<R>;
    type UpdateByIdOutput = R;
    type DeleteOutput = bool;
    type DeleteWithResultsOutput = Vec<R>;
    type DeleteOneWithResultOutput = R;

    async fn insert(
        &self,
        conn: &PgPooledConn,
        query: Q,
        debug_values: String,
    ) -> CustomResult<Self::InsertOutput, errors::DatabaseError>
    where
        Q: AsQuery + QueryFragment<Pg> + RunQueryDsl<PgConnection>,
    {
        match query.get_result_async(conn).await {
            Ok(value) => Ok(value),
            Err(error) => match error {
                ConnectionError::Query(DieselError::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                )) => Err(report!(error)).change_context(errors::DatabaseError::UniqueViolation),
                _ => Err(report!(error)).change_context(errors::DatabaseError::Others),
            }
            .attach_printable_lazy(|| format!("Error while inserting {}", debug_values)),
        }
    }

    async fn update(
        &self,
        conn: &PgPooledConn,
        query: Q,
        debug_values: String,
    ) -> CustomResult<Self::UpdateOutput, errors::DatabaseError>
    where
        Q: QueryId,
    {
        query
            .execute_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable_lazy(|| format!("Error while updating {}", debug_values))
    }

    async fn update_with_results(
        &self,
        conn: &PgPooledConn,
        query: Q,
        debug_values: String,
    ) -> CustomResult<Self::UpdateWithResultsOutput, errors::DatabaseError> {
        query
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable_lazy(|| format!("Error while updating {}", debug_values))
    }

    async fn update_by_id(
        &self,
        conn: &PgPooledConn,
        query: Q,
        debug_values: String,
    ) -> CustomResult<Self::UpdateByIdOutput, errors::DatabaseError>
    where
        Q: Clone,
    {
        // Cloning query for calling `debug_query` later
        match query.to_owned().get_result_async(conn).await {
            Ok(result) => {
                logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());
                Ok(result)
            }
            Err(error) => match error {
                // Failed to generate query, no fields were provided to be updated
                ConnectionError::Query(DieselError::QueryBuilderError(_)) => {
                    Err(report!(error)).change_context(errors::DatabaseError::NoFieldsToUpdate)
                }
                ConnectionError::Query(DieselError::NotFound) => {
                    Err(report!(error)).change_context(errors::DatabaseError::NotFound)
                }
                _ => Err(report!(error)).change_context(errors::DatabaseError::Others),
            }
            .attach_printable_lazy(|| format!("Error while updating by ID {}", debug_values)),
        }
    }

    async fn delete(
        &self,
        conn: &PgPooledConn,
        query: Q,
    ) -> CustomResult<Self::DeleteOutput, errors::DatabaseError>
    where
        Q: QueryId,
    {
        query
            .execute_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error while deleting")
            .and_then(|result| match result {
                n if n > 0 => {
                    logger::debug!("{n} records deleted");
                    Ok(true)
                }
                0 => {
                    Err(report!(errors::DatabaseError::NotFound)
                        .attach_printable("No records deleted"))
                }
                _ => Ok(true), // n is usize, rustc requires this for exhaustive check
            })
    }

    async fn delete_with_results(
        &self,
        conn: &PgPooledConn,
        query: Q,
    ) -> CustomResult<Self::DeleteWithResultsOutput, errors::DatabaseError> {
        query
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error while deleting")
    }

    async fn delete_one_with_result(
        &self,
        conn: &PgPooledConn,
        query: Q,
    ) -> CustomResult<Self::DeleteOneWithResultOutput, errors::DatabaseError> {
        match query.get_result_async(conn).await {
            Ok(value) => Ok(value),
            Err(error) => match error {
                ConnectionError::Query(DieselError::NotFound) => {
                    Err(report!(error)).change_context(errors::DatabaseError::NotFound)
                }
                _ => Err(report!(error)).change_context(errors::DatabaseError::Others),
            }
            .attach_printable("Error while deleting"),
        }
    }
}

#[async_trait]
impl<Q> QueryExecutionMode<Q> for RawQuery
where
    Q: QueryFragment<Pg> + Send + 'static,
{
    type InsertOutput = RawSqlQuery;
    type UpdateOutput = RawSqlQuery;
    type UpdateWithResultsOutput = RawSqlQuery;
    type UpdateByIdOutput = RawSqlQuery;
    type DeleteOutput = RawSqlQuery;
    type DeleteWithResultsOutput = RawSqlQuery;
    type DeleteOneWithResultOutput = RawSqlQuery;

    async fn insert(
        &self,
        _conn: &PgPooledConn,
        query: Q,
        _debug_values: String,
    ) -> CustomResult<Self::InsertOutput, errors::DatabaseError>
    where
        Q: AsQuery + QueryFragment<Pg> + RunQueryDsl<PgConnection>,
    {
        generate_raw_query(query)
    }

    async fn update(
        &self,
        _conn: &PgPooledConn,
        query: Q,
        _debug_values: String,
    ) -> CustomResult<Self::UpdateOutput, errors::DatabaseError>
    where
        Q: QueryId,
    {
        generate_raw_query(query)
    }

    async fn update_with_results(
        &self,
        _conn: &PgPooledConn,
        query: Q,
        _debug_values: String,
    ) -> CustomResult<Self::UpdateWithResultsOutput, errors::DatabaseError> {
        generate_raw_query(query)
    }

    async fn update_by_id(
        &self,
        _conn: &PgPooledConn,
        query: Q,
        _debug_values: String,
    ) -> CustomResult<Self::UpdateByIdOutput, errors::DatabaseError>
    where
        Q: Clone,
    {
        generate_raw_query(query)
    }

    async fn delete(
        &self,
        _conn: &PgPooledConn,
        query: Q,
    ) -> CustomResult<Self::DeleteOutput, errors::DatabaseError>
    where
        Q: QueryId,
    {
        generate_raw_query(query)
    }

    async fn delete_with_results(
        &self,
        _conn: &PgPooledConn,
        query: Q,
    ) -> CustomResult<Self::DeleteWithResultsOutput, errors::DatabaseError> {
        generate_raw_query(query)
    }

    async fn delete_one_with_result(
        &self,
        _conn: &PgPooledConn,
        query: Q,
    ) -> CustomResult<Self::DeleteOneWithResultOutput, errors::DatabaseError> {
        generate_raw_query(query)
    }
}

pub fn generate_raw_query<Q>(query: Q) -> CustomResult<RawSqlQuery, errors::DatabaseError>
where
    Q: QueryFragment<Pg>,
{
    let raw_query = diesel::query_builder::raw_query(&query)
        .into_report()
        .change_context(errors::DatabaseError::QueryGenerationFailed)?;

    Ok(RawSqlQuery {
        sql: raw_query.raw_sql,
        binds: raw_query.raw_binds,
    })
}

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_insert<T, V, R, Q>(
    conn: &PgPooledConn,
    values: V,
    execution_mode: Q,
) -> CustomResult<Q::InsertOutput, errors::DatabaseError>
where
    T: HasTable<Table = T> + Table + 'static,
    V: Debug + Insertable<T>,
    <T as QuerySource>::FromClause: QueryFragment<Pg> + Send,
    <V as Insertable<T>>::Values: CanInsertInSingleQuery<Pg> + QueryFragment<Pg> + 'static,
    InsertStatement<T, <V as Insertable<T>>::Values>:
        AsQuery + LoadQuery<'static, PgConnection, R> + Clone + Send,
    R: Send + 'static,

    Q: QueryExecutionMode<InsertStatement<T, <V as Insertable<T>>::Values>>,
{
    let debug_values = format!("{:?}", values);

    let query = diesel::insert_into(<T as HasTable>::table()).values(values);
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    execution_mode.insert(conn, query, debug_values).await
}

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_update<T, V, P, Q>(
    conn: &PgPooledConn,
    predicate: P,
    values: V,
    execution_mode: Q,
) -> CustomResult<Q::UpdateOutput, errors::DatabaseError>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    V: AsChangeset<Target = <<T as FilterDsl<P>>::Output as HasTable>::Table> + Debug,
    <T as FilterDsl<P>>::Output: IntoUpdateTarget,
    UpdateStatement<
        <<T as FilterDsl<P>>::Output as HasTable>::Table,
        <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
        <V as AsChangeset>::Changeset,
    >: AsQuery + QueryFragment<Pg> + QueryId + Send + 'static,

    Q: QueryExecutionMode<
        UpdateStatement<
            <<T as FilterDsl<P>>::Output as HasTable>::Table,
            <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
            <V as AsChangeset>::Changeset,
        >,
    >,
{
    let debug_values = format!("{:?}", values);

    let query = diesel::update(<T as HasTable>::table().filter(predicate)).set(values);
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    execution_mode.update(conn, query, debug_values).await
}

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_update_with_results<T, V, P, R, Q>(
    conn: &PgPooledConn,
    predicate: P,
    values: V,
    execution_mode: Q,
) -> CustomResult<Q::UpdateWithResultsOutput, errors::DatabaseError>
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

    Q: QueryExecutionMode<
        UpdateStatement<
            <<T as FilterDsl<P>>::Output as HasTable>::Table,
            <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
            <V as AsChangeset>::Changeset,
        >,
    >,
{
    let debug_values = format!("{:?}", values);

    let query = diesel::update(<T as HasTable>::table().filter(predicate)).set(values);
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    execution_mode
        .update_with_results(conn, query, debug_values)
        .await
}

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_update_by_id<T, V, Pk, R, Q>(
    conn: &PgPooledConn,
    id: Pk,
    values: V,
    execution_mode: Q,
) -> CustomResult<Q::UpdateByIdOutput, errors::DatabaseError>
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

    Q: QueryExecutionMode<
        UpdateStatement<
            <<T as FindDsl<Pk>>::Output as HasTable>::Table,
            <<T as FindDsl<Pk>>::Output as IntoUpdateTarget>::WhereClause,
            <V as AsChangeset>::Changeset,
        >,
    >,
{
    let debug_values = format!("{:?}", values);

    let query = diesel::update(<T as HasTable>::table().find(id.to_owned())).set(values);

    execution_mode.update_by_id(conn, query, debug_values).await
}

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_delete<T, P, Q>(
    conn: &PgPooledConn,
    predicate: P,
    execution_mode: Q,
) -> CustomResult<Q::DeleteOutput, errors::DatabaseError>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    <T as FilterDsl<P>>::Output: IntoUpdateTarget,
    DeleteStatement<
        <<T as FilterDsl<P>>::Output as HasTable>::Table,
        <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
    >: AsQuery + QueryFragment<Pg> + QueryId + Send + 'static,

    Q: QueryExecutionMode<
        DeleteStatement<
            <<T as FilterDsl<P>>::Output as HasTable>::Table,
            <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
        >,
    >,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    execution_mode.delete(conn, query).await
}

#[allow(dead_code)]
#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_delete_with_results<T, P, R, Q>(
    conn: &PgPooledConn,
    predicate: P,
    execution_mode: Q,
) -> CustomResult<Q::DeleteWithResultsOutput, errors::DatabaseError>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    <T as FilterDsl<P>>::Output: IntoUpdateTarget,
    DeleteStatement<
        <<T as FilterDsl<P>>::Output as HasTable>::Table,
        <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
    >: AsQuery + LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static,

    Q: QueryExecutionMode<
        DeleteStatement<
            <<T as FilterDsl<P>>::Output as HasTable>::Table,
            <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
        >,
    >,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    execution_mode.delete_with_results(conn, query).await
}

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_delete_one_with_result<T, P, R, Q>(
    conn: &PgPooledConn,
    predicate: P,
    execution_mode: Q,
) -> CustomResult<Q::DeleteOneWithResultOutput, errors::DatabaseError>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    <T as FilterDsl<P>>::Output: IntoUpdateTarget,
    DeleteStatement<
        <<T as FilterDsl<P>>::Output as HasTable>::Table,
        <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
    >: AsQuery + LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + Clone + 'static,

    Q: QueryExecutionMode<
        DeleteStatement<
            <<T as FilterDsl<P>>::Output as HasTable>::Table,
            <<T as FilterDsl<P>>::Output as IntoUpdateTarget>::WhereClause,
        >,
    >,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

    execution_mode.delete_one_with_result(conn, query).await
}

#[instrument(level = "DEBUG", skip_all)]
async fn generic_find_by_id_core<T, Pk, R>(
    conn: &PgPooledConn,
    id: Pk,
) -> CustomResult<R, errors::DatabaseError>
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
pub async fn generic_find_by_id<T, Pk, R>(
    conn: &PgPooledConn,
    id: Pk,
) -> CustomResult<R, errors::DatabaseError>
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
pub async fn generic_find_by_id_optional<T, Pk, R>(
    conn: &PgPooledConn,
    id: Pk,
) -> CustomResult<Option<R>, errors::DatabaseError>
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
async fn generic_find_one_core<T, P, R>(
    conn: &PgPooledConn,
    predicate: P,
) -> CustomResult<R, errors::DatabaseError>
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
pub async fn generic_find_one<T, P, R>(
    conn: &PgPooledConn,
    predicate: P,
) -> CustomResult<R, errors::DatabaseError>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    <T as FilterDsl<P>>::Output:
        LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static,
{
    generic_find_one_core::<T, _, _>(conn, predicate).await
}

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_find_one_optional<T, P, R>(
    conn: &PgPooledConn,
    predicate: P,
) -> CustomResult<Option<R>, errors::DatabaseError>
where
    T: FilterDsl<P> + HasTable<Table = T> + Table + 'static,
    <T as FilterDsl<P>>::Output:
        LoadQuery<'static, PgConnection, R> + QueryFragment<Pg> + Send + 'static,
    R: Send + 'static,
{
    to_optional(generic_find_one_core::<T, _, _>(conn, predicate).await)
}

#[instrument(level = "DEBUG", skip_all)]
pub async fn generic_filter<T, P, R>(
    conn: &PgPooledConn,
    predicate: P,
    limit: Option<i64>,
) -> CustomResult<Vec<R>, errors::DatabaseError>
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

pub fn to_optional<T>(
    arg: CustomResult<T, errors::DatabaseError>,
) -> CustomResult<Option<T>, errors::DatabaseError> {
    match arg {
        Ok(value) => Ok(Some(value)),
        Err(err) => match err.current_context() {
            errors::DatabaseError::NotFound => Ok(None),
            _ => Err(err),
        },
    }
}
