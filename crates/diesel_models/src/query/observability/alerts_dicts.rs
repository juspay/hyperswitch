use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, query_builder::DecoratableTarget, sql_types::Bool, upsert::excluded,
    BoolExpressionMethods, ExpressionMethods,
};
use error_stack::{report, ResultExt};

use crate::{
    errors,
    observability::{
        alerts_dicts::{AlertsDict, AlertsDictNew, AlertsDictRetire},
        schema::alerts_dicts::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

const ENABLED_INDEX_PREDICATE: &str = "is_enabled IS TRUE";

impl AlertsDictNew {
    pub async fn upsert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<AlertsDict> {
        let query = diesel::insert_into(<AlertsDict as HasTable>::table())
            .values(self)
            .on_conflict((dsl::name, dsl::key_))
            .filter_target(diesel::dsl::sql::<Bool>(ENABLED_INDEX_PREDICATE))
            .do_update()
            .set((
                dsl::product.eq(excluded(dsl::product)),
                dsl::values_.eq(excluded(dsl::values_)),
                dsl::ts_created.eq(excluded(dsl::ts_created)),
                dsl::username.eq(excluded(dsl::username)),
                dsl::metadata.eq(excluded(dsl::metadata)),
            ));

        generics::db_metrics::track_database_call::<<AlertsDict as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Insert,
            query.get_result_async(conn.raw_connection()),
        )
        .await
        .map_err(|error| match error {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => report!(error).change_context(errors::DatabaseError::UniqueViolation),
            _ => report!(error).change_context(errors::DatabaseError::Others),
        })
        .attach_printable("Error while saving a dictionary entry")
    }
}

impl AlertsDict {
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
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::name
                .eq(name.to_owned())
                .and(dsl::key_.eq(key.to_owned()))
                .and(dsl::is_enabled.eq(true)),
        )
        .await
    }

    pub async fn retire(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        key: &str,
    ) -> StorageResult<Option<Self>> {
        let retired =
            generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, Self>(
                conn,
                dsl::name
                    .eq(name.to_owned())
                    .and(dsl::key_.eq(key.to_owned()))
                    .and(dsl::is_enabled.eq(true)),
                AlertsDictRetire {
                    is_enabled: Some(false),
                },
            )
            .await?;

        Ok(retired.into_iter().next())
    }
}
