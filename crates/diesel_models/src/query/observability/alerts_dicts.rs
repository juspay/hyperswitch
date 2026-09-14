use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable,
    sql_types::{Integer, Text},
    BoolExpressionMethods, ExpressionMethods, PgSortExpressionMethods, QueryDsl,
};
use error_stack::ResultExt;

use crate::{
    errors,
    observability::{
        alerts_dicts::{AlertsDict, AlertsDictNew, AlertsDictUpdate, AlertsDictUpdateInternal},
        schema::alerts_dicts::{self, dsl},
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

const MAPPER_ENTRY_LOCK_NAMESPACE: i32 = 23_403;

diesel::alias!(alerts_dicts as superseded_dicts: SupersededDicts);

impl AlertsDictNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<AlertsDict> {
        generics::generic_insert(conn, self).await
    }
}

impl AlertsDict {
    pub async fn lock_by_name_and_key(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        key: &str,
    ) -> StorageResult<()> {
        let query = diesel::sql_query("SELECT pg_advisory_xact_lock($1, hashtext($2))")
            .bind::<Integer, _>(MAPPER_ENTRY_LOCK_NAMESPACE)
            .bind::<Text, _>(format!("{name}/{key}"));

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Update,
            query.execute_async(conn.raw_connection()),
        )
        .await
        .map(|_| ())
        .map_err(|error| error_stack::report!(error))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to lock a mapper entry")
    }

    pub async fn list_enabled(
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
            conn,
            dsl::is_enabled.eq(true),
            None,
            None,
            Some((dsl::name.asc(), dsl::key_.asc())),
        )
        .await
    }

    pub async fn find_enabled_by_name_and_key(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        key: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::name
                .eq(name.to_owned())
                .and(dsl::key_.eq(key.to_owned()))
                .and(dsl::is_enabled.eq(true)),
        )
        .await
    }

    pub async fn update_enabled_by_name_and_key(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        key: &str,
        update: AlertsDictUpdate,
    ) -> StorageResult<usize> {
        generics::generic_update::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::name
                .eq(name.to_owned())
                .and(dsl::key_.eq(key.to_owned()))
                .and(dsl::is_enabled.eq(true)),
            AlertsDictUpdateInternal::from(update),
        )
        .await
    }

    pub async fn delete_enabled_by_name_and_key(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        key: &str,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::name
                .eq(name.to_owned())
                .and(dsl::key_.eq(key.to_owned()))
                .and(dsl::is_enabled.eq(true)),
        )
        .await
    }

    pub async fn delete_superseded_by_name_and_key(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        key: &str,
        kept: i64,
    ) -> StorageResult<usize> {
        let superseded = superseded_dicts
            .select(superseded_dicts.field(dsl::id))
            .filter(
                superseded_dicts
                    .field(dsl::name)
                    .eq(name.to_owned())
                    .and(superseded_dicts.field(dsl::key_).eq(key.to_owned()))
                    .and(superseded_dicts.field(dsl::is_enabled).eq(false)),
            )
            .order((
                superseded_dicts.field(dsl::ts_created).desc().nulls_last(),
                superseded_dicts.field(dsl::id).desc(),
            ))
            .offset(kept);

        let query = diesel::delete(<Self as HasTable>::table().filter(dsl::id.eq_any(superseded)));

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Delete,
            query.execute_async(conn.raw_connection()),
        )
        .await
        .map_err(|error| error_stack::report!(error))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to delete superseded mapper entries")
    }
}
