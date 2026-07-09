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
    (@op "generic_find_by_id_core") => {
        deja::OperationKind::Read
    };
    (@op "generic_find_one_core") => {
        deja::OperationKind::Read
    };
    (@op "generic_filter") => {
        deja::OperationKind::Read
    };
    (@op "generic_count") => {
        deja::OperationKind::Read
    };
    (@op "generic_insert") => {
        deja::OperationKind::Create
    };
    (@op "generic_update") => {
        deja::OperationKind::Update
    };
    (@op "generic_delete") => {
        deja::OperationKind::Delete
    };
    (@op "generic_update_with_results") => {
        deja::OperationKind::Update
    };
    (@op "generic_update_by_id") => {
        deja::OperationKind::Update
    };
    (@op "generic_delete_one_with_result") => {
        deja::OperationKind::Delete
    };
    (@op $operation:tt) => {
        compile_error!("unclassified Deja db operation")
    };
    (@state $spec:expr, $key:expr, "generic_find_by_id_core") => {
        $spec.state_read_to($key)
    };
    (@state $spec:expr, $key:expr, "generic_find_one_core") => {
        $spec.state_read_to($key)
    };
    (@state $spec:expr, $key:expr, "generic_filter") => {
        $spec.state_read_to($key)
    };
    (@state $spec:expr, $key:expr, "generic_count") => {
        $spec.state_read_to($key)
    };
    (@state $spec:expr, $key:expr, "generic_insert") => {
        $spec.state_write_to($key)
    };
    (@state $spec:expr, $key:expr, "generic_update") => {
        $spec.state_write_to($key)
    };
    (@state $spec:expr, $key:expr, "generic_delete") => {
        $spec.state_write_to($key)
    };
    (@state $spec:expr, $key:expr, "generic_update_with_results") => {
        $spec.state_touch_to($key)
    };
    (@state $spec:expr, $key:expr, "generic_update_by_id") => {
        $spec.state_touch_to($key)
    };
    (@state $spec:expr, $key:expr, "generic_delete_one_with_result") => {
        $spec.state_touch_to($key)
    };
    (@state $spec:expr, $key:expr, $operation:tt) => {
        compile_error!("unclassified Deja db operation")
    };
    (@returns $spec:expr, "generic_update_with_results") => {
        $spec.return_semantics(deja::ReturnSemantics::UpdateReturning)
    };
    (@returns $spec:expr, "generic_update_by_id") => {
        $spec.return_semantics(deja::ReturnSemantics::UpdateReturning)
    };
    (@returns $spec:expr, "generic_delete_one_with_result") => {
        $spec.return_semantics(deja::ReturnSemantics::DeleteReturning)
    };
    (@returns $spec:expr, $operation:tt) => {
        $spec
    };
    ($operation:tt, $table:expr, $sql:expr, $inputs:expr, $kind:expr, $body:block) => {{
        let __deja_operation = $operation;
        let __deja_table = $table;
        let __deja_sql = $sql;
        let __deja_inputs = $inputs;
        let __deja_result_kind = $kind;
        let __deja_state_key =
            deja::db::query_state_key(__deja_operation, __deja_table, &__deja_sql, &__deja_inputs);
        let __deja_spec =
            deja::db::QuerySpec::new(__deja_operation, __deja_table, __deja_sql, __deja_inputs)
                .component("diesel_models::query::generics")
                .operation_kind(record_deja_db_query!(@op $operation));
        let __deja_spec = record_deja_db_query!(@state __deja_spec, __deja_state_key, $operation);
        let mut __deja_spec = record_deja_db_query!(@returns __deja_spec, $operation);
        if matches!(
            __deja_result_kind,
            deja::db::QueryResultKind::Value
                | deja::db::QueryResultKind::Rows
                | deja::db::QueryResultKind::Optional
        ) {
            __deja_spec.declaration = ::std::mem::take(&mut __deja_spec.declaration)
                .state_canon(deja::CanonRef::new(
                    "project:!created_at,!last_synced,!modified_at",
                ));
        }
        let __deja_caller = ::std::panic::Location::caller();

        async move {
            let deja::db::QuerySpec {
                boundary: __deja_boundary,
                component: __deja_component,
                operation: __deja_operation,
                table: __deja_table,
                sql: __deja_sql,
                inputs: __deja_inputs,
                correlation_id: __deja_correlation_id,
                read_set: __deja_read_set,
                write_set: __deja_write_set,
                declaration: __deja_declaration,
            } = __deja_spec;

            // Build the lookup args once; the exact same envelope feeds replay
            // lookup and event recording.
            let __deja_request =
                deja::db::args(__deja_operation, &__deja_table, __deja_sql, __deja_inputs);

            // Derive the syntactic identity (boundary/component/operation) and
            // allocate the occurrence exactly once before dispatch, so replay
            // lookup and event recording stamp identical callsite identities.
            let __deja_scope = format!("{}::{}", __deja_component, __deja_operation);
            let __deja_syntax_hash = deja::__private::stable_callsite_hash(&format!(
                "{}::{}::{}",
                __deja_boundary, __deja_component, __deja_operation
            ));
            let __deja_occurrence = deja::__private::next_boundary_occurrence(
                __deja_correlation_id.as_deref(),
                deja::__private::CallsiteSource::SyntacticHash,
                Some(__deja_scope.as_str()),
            );
            let __deja_identity = deja::__private::CallsiteIdentity {
                version: 1,
                source: deja::__private::CallsiteSource::SyntacticHash,
                id: None,
                scope: Some(__deja_scope.clone()),
                occurrence: __deja_occurrence,
                caller_function: Some(__deja_component.to_string()),
                lexical_path: Some(__deja_scope.clone()),
                syntax_hash: Some(__deja_syntax_hash),
                span_path: deja::__private::current_span_path(),
            };

            let mut __deja_declaration = __deja_declaration;
            if __deja_declaration.effect.is_none() {
                __deja_declaration.effect = Some(deja::EffectKind::Db);
            }
            if __deja_declaration.returns.is_none() {
                __deja_declaration.returns = Some(match __deja_result_kind {
                    deja::db::QueryResultKind::Value => deja::ReturnSemantics::Value,
                    deja::db::QueryResultKind::Rows => deja::ReturnSemantics::Rows,
                    deja::db::QueryResultKind::Optional => deja::ReturnSemantics::Optional,
                    deja::db::QueryResultKind::Count => deja::ReturnSemantics::Count,
                    deja::db::QueryResultKind::Bool => deja::ReturnSemantics::Bool,
                    deja::db::QueryResultKind::Unit => deja::ReturnSemantics::Unit,
                });
            }
            let __deja_semantics = deja::__private::BoundarySemantics {
                replay_strategy: deja::ReplayStrategy::Execute,
                kind: Some("db".to_string()),
                declaration: (!__deja_declaration.is_empty()).then_some(__deja_declaration),
            };
            let __deja_boundary_spec = deja::__private::BoundarySpec::with_semantics(
                __deja_boundary,
                __deja_component,
                __deja_operation,
                __deja_semantics,
            );

            // Db replay fall-through: a lookup hit that cannot be decoded, or
            // an error kind this callsite does not recover, runs the live
            // query without re-recording.
            let mut __deja_obs = deja::__private::CrossingObservation::with_correlation(
                __deja_boundary_spec,
                __deja_identity,
                __deja_caller,
                __deja_correlation_id,
            )
            .fall_through_silent();
            if !__deja_read_set.is_empty() {
                __deja_obs = __deja_obs.with_read_set(__deja_read_set);
            }
            if !__deja_write_set.is_empty() {
                __deja_obs = __deja_obs.with_write_set(__deja_write_set);
            }

            deja::__private::dispatch_async(
                __deja_obs,
                move || __deja_request,
                move || async move {
                    $body
                },
                move |__deja_recorded| {
                    // Recorded DB results always carry the versioned
                    // DejaDatabaseResult envelope (result_serialize_db); anything
                    // else is malformed capture data and must fail-stop, never
                    // silently run live.
                    match serde_json::from_value::<deja::value::DejaDatabaseResult>(
                        __deja_recorded,
                    ) {
                        Ok(__deja_structured) => match __deja_structured.payload {
                            deja::value::DejaDatabaseResultPayload::Ok {
                                value: __deja_value,
                                ..
                            } => match serde_json::from_value(__deja_value) {
                                Ok(__deja_typed) => {
                                    deja::__private::Reconstructed::Value(Ok(__deja_typed))
                                }
                                // A faithfully recorded Ok that no longer fits the
                                // candidate's row type: reconstruction FAILURE.
                                Err(_) => deja::__private::Reconstructed::Failed,
                            },
                            deja::value::DejaDatabaseResultPayload::Err {
                                kind: __deja_kind,
                                message: __deja_message,
                            } => {
                                let _ = __deja_message;
                                match __deja_kind.as_str() {
                                    "NotFound" => deja::__private::Reconstructed::Value(Err(
                                        error_stack::report!(errors::DatabaseError::NotFound),
                                    )),
                                    "UniqueViolation" => {
                                        deja::__private::Reconstructed::Value(Err(
                                            error_stack::report!(
                                                errors::DatabaseError::UniqueViolation
                                            ),
                                        ))
                                    }
                                    // A recorded error whose concrete shape is not
                                    // reconstructable ("Other"): fail-stop rather
                                    // than silently rerunning the live query.
                                    _ => deja::__private::Reconstructed::Failed,
                                }
                            }
                        },
                        Err(_) => deja::__private::Reconstructed::Failed,
                    }
                },
                move |__deja_output| {
                    let (__deja_result_json, __deja_is_error) = deja::value::result_serialize_db(
                        __deja_output,
                        |__deja_err: &error_stack::Report<errors::DatabaseError>|
                            -> (::std::string::String, ::std::string::String) {
                            let __deja_kind = match __deja_err.current_context() {
                                errors::DatabaseError::NotFound => "NotFound",
                                errors::DatabaseError::UniqueViolation => "UniqueViolation",
                                _ => "Other",
                            };
                            (
                                __deja_kind.to_string(),
                                format!("{__deja_err:?}"),
                            )
                        },
                    );
                    let mut __deja_capture =
                        deja::RecordedOutput::new(__deja_result_json, __deja_is_error);
                    if !__deja_is_error {
                        if let Ok(__deja_structured) =
                            serde_json::from_value::<deja::value::DejaDatabaseResult>(
                                __deja_capture.result.clone(),
                            )
                        {
                            if let deja::value::DejaDatabaseResultPayload::Ok {
                                value: __deja_value,
                                ..
                            } = __deja_structured.payload
                            {
                                for __deja_row_key in deja::db::row_state_keys(&__deja_table, &__deja_value) {
                                    let __deja_row_key = __deja_row_key.to_wire();
                                    match __deja_operation {
                                        "generic_find_by_id_core"
                                        | "generic_find_one_core"
                                        | "generic_filter"
                                        | "generic_count" => {
                                            __deja_capture =
                                                __deja_capture.with_read_key(__deja_row_key);
                                        }
                                        "generic_update_with_results"
                                        | "generic_update_by_id"
                                        | "generic_delete_one_with_result" => {
                                            __deja_capture = __deja_capture
                                                .with_read_key(__deja_row_key.clone())
                                                .with_write_key(__deja_row_key);
                                        }
                                        _ => {
                                            __deja_capture =
                                                __deja_capture.with_write_key(__deja_row_key);
                                        }
                                    }
                                }
                                if let Some(__deja_image) =
                                    deja::db::row_image_payload(&__deja_table, &__deja_value)
                                {
                                    __deja_capture =
                                        __deja_capture.with_result_image(__deja_image);
                                }
                            }
                        }
                    }
                    __deja_capture
                },
            )
            .await
        }
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
pub trait DejaQueryResult: Debug + serde::Serialize + serde::de::DeserializeOwned {}
#[cfg(feature = "deja")]
impl<T: Debug + serde::Serialize + serde::de::DeserializeOwned> DejaQueryResult for T {}
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
    R: Send + 'static + DejaQueryResult,
{
    let debug_values = format!("{values:?}");

    let query = diesel::insert_into(<T as HasTable>::table()).values(values);
    #[cfg(feature = "deja")]
    let sql = debug_query::<Pg, _>(&query).to_string();
    #[cfg(feature = "deja")]
    logger::debug!(query = %sql);
    #[cfg(not(feature = "deja"))]
    logger::debug!(query = %debug_query::<Pg, _>(&query));
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
    #[cfg(feature = "deja")]
    let sql = debug_query::<Pg, _>(&query).to_string();
    #[cfg(feature = "deja")]
    logger::debug!(query = %sql);
    #[cfg(not(feature = "deja"))]
    logger::debug!(query = %debug_query::<Pg, _>(&query));
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
    R: Send + 'static + DejaQueryResult,

    // For cloning query (UpdateStatement)
    <Filter<T, P> as HasTable>::Table: Clone,
    <Filter<T, P> as IntoUpdateTarget>::WhereClause: Clone,
    <V as AsChangeset>::Changeset: Clone,
    <<Filter<T, P> as HasTable>::Table as QuerySource>::FromClause: Clone,
{
    let debug_values = format!("{values:?}");

    let query = diesel::update(<T as HasTable>::table().filter(predicate)).set(values);
    #[cfg(feature = "deja")]
    let sql = debug_query::<Pg, _>(&query).to_string();
    #[cfg(feature = "deja")]
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
                Ok(result) => {
                    #[cfg(not(feature = "deja"))]
                    logger::debug!(query = %debug_query::<Pg, _>(&query));
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
    #[cfg(feature = "deja")]
    let sql = debug_query::<Pg, _>(&query).to_string();
    #[cfg(feature = "deja")]
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
                    #[cfg(not(feature = "deja"))]
                    logger::debug!(query = %debug_query::<Pg, _>(&query));
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
    #[cfg(feature = "deja")]
    let sql = debug_query::<Pg, _>(&query).to_string();
    #[cfg(feature = "deja")]
    logger::debug!(query = %sql);
    #[cfg(not(feature = "deja"))]
    logger::debug!(query = %debug_query::<Pg, _>(&query));
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
    R: Send + Clone + 'static + DejaQueryResult,
{
    let query = diesel::delete(<T as HasTable>::table().filter(predicate));
    #[cfg(feature = "deja")]
    let sql = debug_query::<Pg, _>(&query).to_string();
    #[cfg(feature = "deja")]
    logger::debug!(query = %sql);
    #[cfg(not(feature = "deja"))]
    logger::debug!(query = %debug_query::<Pg, _>(&query));
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
    R: Send + 'static + DejaQueryResult,
{
    let query = <T as HasTable>::table().find(id.to_owned());
    #[cfg(feature = "deja")]
    let sql = debug_query::<Pg, _>(&query).to_string();
    #[cfg(feature = "deja")]
    logger::debug!(query = %sql);
    #[cfg(not(feature = "deja"))]
    logger::debug!(query = %debug_query::<Pg, _>(&query));
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
    #[cfg(feature = "deja")]
    let sql = debug_query::<Pg, _>(&query).to_string();
    #[cfg(feature = "deja")]
    logger::debug!(query = %sql);
    #[cfg(not(feature = "deja"))]
    logger::debug!(query = %debug_query::<Pg, _>(&query));
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

    #[cfg(feature = "deja")]
    let sql = debug_query::<Pg, _>(&query).to_string();
    #[cfg(feature = "deja")]
    logger::debug!(query = %sql);
    #[cfg(not(feature = "deja"))]
    logger::debug!(query = %debug_query::<Pg, _>(&query));
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

    #[cfg(feature = "deja")]
    let sql = debug_query::<Pg, _>(&query).to_string();
    #[cfg(feature = "deja")]
    logger::debug!(query = %sql);
    #[cfg(not(feature = "deja"))]
    logger::debug!(query = %debug_query::<Pg, _>(&query));
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
