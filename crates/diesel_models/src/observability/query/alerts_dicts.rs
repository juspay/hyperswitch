use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, debug_query, pg::Pg, BoolExpressionMethods, ExpressionMethods, QueryDsl,
};
use error_stack::ResultExt;
use router_env::logger;

use crate::{
    errors::DatabaseError,
    observability::{
        alerts_dicts::{AlertsDicts, AlertsDictsNew},
        schema::alerts_dicts::dsl,
    },
    query::{
        generics,
        generics::db_metrics::{track_database_call, DatabaseOperation},
    },
    DatabaseConnectionWithContext, StorageResult,
};

impl AlertsDictsNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<AlertsDicts> {
        generics::generic_insert(conn, self).await
    }
}

impl AlertsDicts {
    pub async fn demote_enabled_by_name_key(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        key_: &str,
    ) -> StorageResult<usize> {
        generics::generic_update::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::name
                .eq(name.to_owned())
                .and(dsl::key_.eq(key_.to_owned()))
                .and(dsl::is_enabled.eq(true)),
            dsl::is_enabled.eq(false),
        )
        .await
    }

    pub async fn find_superseded_ids_by_name_key(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        key_: &str,
        keep: i64,
    ) -> StorageResult<Vec<uuid::Uuid>> {
        let query = Self::table()
            .select(dsl::id)
            .filter(
                dsl::name
                    .eq(name.to_owned())
                    .and(dsl::key_.eq(key_.to_owned())),
            )
            .order((dsl::ts_created.desc(), dsl::id.desc()))
            .offset(keep);

        logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        track_database_call::<Self, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            DatabaseOperation::Filter,
            query.get_results_async(conn.raw_connection()),
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable("Error finding superseded alerts_dicts versions")
    }

    pub async fn delete_by_ids(
        conn: &DatabaseConnectionWithContext<'_>,
        ids: Vec<uuid::Uuid>,
    ) -> StorageResult<usize> {
        let query = diesel::delete(Self::table().filter(dsl::id.eq_any(ids)));

        logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        track_database_call::<Self, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            DatabaseOperation::Delete,
            query.execute_async(conn.raw_connection()),
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable("Error deleting superseded alerts_dicts versions")
    }

    pub async fn find_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: uuid::Uuid,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, id).await
    }

    pub async fn list_by_filter(
        conn: &DatabaseConnectionWithContext<'_>,
        name: Option<String>,
        key_: Option<String>,
        is_enabled: bool,
    ) -> StorageResult<Vec<Self>> {
        let mut query =
            crate::list::into_boxed_list(Self::table().filter(dsl::is_enabled.eq(is_enabled)));

        if let Some(name) = name {
            query = query.filter(dsl::name.eq(name));
        }

        if let Some(key_) = key_ {
            query = query.filter(dsl::key_.eq(key_));
        }

        logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        track_database_call::<Self, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            DatabaseOperation::Filter,
            query.get_results_async(conn.raw_connection()),
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable("Error filtering alerts_dicts")
    }

    pub async fn delete_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: uuid::Uuid,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(conn, dsl::id.eq(id)).await
    }
}
