use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable,
    debug_query,
    expression_methods::{PgAnyJsonExpressionMethods, PgJsonbExpressionMethods},
    pg::Pg,
    sql_types::{Bool, Nullable},
    BoolExpressionMethods, BoxableExpression, ExpressionMethods, NullableExpressionMethods,
    QueryDsl,
};
use error_stack::report;
use router_env::logger;

use crate::{
    errors::DatabaseError,
    observability::{
        alert_manager::merchant_thresholds::{
            MerchantThresholds, MerchantThresholdsFilter, MerchantThresholdsKeyFilter,
            MerchantThresholdsNew, MerchantThresholdsUpdate,
        },
        schema::merchant_thresholds::{self, dsl},
    },
    query::generics::{
        self,
        db_metrics::{track_database_call, DatabaseOperation},
    },
    DatabaseConnectionWithContext, StorageResult,
};

impl MerchantThresholdsNew {
    pub async fn upsert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
        on_conflict: Option<MerchantThresholdsUpdate>,
    ) -> StorageResult<Vec<MerchantThresholds>> {
        let conflict_target = (
            dsl::name,
            dsl::product,
            dsl::merchant_id,
            dsl::profile_id,
            dsl::is_enabled,
            dsl::author,
        );

        match on_conflict {
            Some(update) => {
                let query = diesel::insert_into(<MerchantThresholds as HasTable>::table())
                    .values(self)
                    .on_conflict(conflict_target)
                    .do_update()
                    .set(update);

                logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

                track_database_call::<MerchantThresholds, _, _>(
                    conn.request_id(),
                    conn.event_emitter(),
                    DatabaseOperation::Insert,
                    query.get_results_async(conn.raw_connection()),
                )
                .await
                .map_err(storage_error)
            }
            None => {
                let query = diesel::insert_into(<MerchantThresholds as HasTable>::table())
                    .values(self)
                    .on_conflict(conflict_target)
                    .do_nothing();

                logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

                track_database_call::<MerchantThresholds, _, _>(
                    conn.request_id(),
                    conn.event_emitter(),
                    DatabaseOperation::Insert,
                    query.get_results_async(conn.raw_connection()),
                )
                .await
                .map_err(storage_error)
            }
        }
    }
}

impl MerchantThresholds {
    pub async fn list_by_filter(
        conn: &DatabaseConnectionWithContext<'_>,
        filter: MerchantThresholdsFilter,
    ) -> StorageResult<Vec<Self>> {
        let mut query = crate::list::into_boxed_list(Self::table());

        if let Some(ids) = filter.ids {
            query = query.filter(dsl::id.eq_any(ids));
        }
        if let Some(names) = filter.names {
            query = query.filter(dsl::name.eq_any(names));
        }
        if let Some(products) = filter.products {
            query = query.filter(dsl::product.eq_any(products));
        }
        if let Some(merchant_ids) = filter.merchant_ids {
            query = query.filter(dsl::merchant_id.eq_any(merchant_ids));
        }
        if let Some(profile_ids) = filter.profile_ids {
            query = query.filter(dsl::profile_id.eq_any(profile_ids));
        }
        if let Some(authors) = filter.authors {
            query = query.filter(dsl::author.eq_any(authors));
        }
        if let Some(is_enabled) = filter.is_enabled {
            query = query.filter(dsl::is_enabled.eq(is_enabled));
        }
        for (key, values) in filter.metadata {
            query = query.filter(dsl::metadata.retrieve_as_text(key).eq_any(values));
        }
        if let Some(from) = filter.updated_from {
            query = query.filter(dsl::last_updated_at.ge(from));
        }
        if let Some(to) = filter.updated_to {
            query = query.filter(dsl::last_updated_at.le(to));
        }

        logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        track_database_call::<Self, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            DatabaseOperation::Filter,
            query.get_results_async(conn.raw_connection()),
        )
        .await
        .map_err(storage_error)
    }

    pub async fn update_by_key_filter(
        conn: &DatabaseConnectionWithContext<'_>,
        filter: MerchantThresholdsKeyFilter,
        update: MerchantThresholdsUpdate,
        metadata_merge: Option<serde_json::Value>,
    ) -> StorageResult<Vec<Self>> {
        let predicate = key_predicate(filter)?;
        let merge = metadata_merge
            .map(|value| dsl::metadata.eq(PgJsonbExpressionMethods::concat(dsl::metadata, value)));

        let query = diesel::update(Self::table().filter(predicate)).set((update, merge));

        logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        track_database_call::<Self, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            DatabaseOperation::UpdateWithResults,
            query.get_results_async(conn.raw_connection()),
        )
        .await
        .map_err(storage_error)
    }

    pub async fn delete_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: String,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq(id),
        )
        .await
    }

    pub async fn delete_by_key_filter(
        conn: &DatabaseConnectionWithContext<'_>,
        filter: MerchantThresholdsKeyFilter,
    ) -> StorageResult<Vec<Self>> {
        let predicate = key_predicate(filter)?;
        let query = diesel::delete(Self::table().filter(predicate));

        logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        track_database_call::<Self, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            DatabaseOperation::DeleteWithResult,
            query.get_results_async(conn.raw_connection()),
        )
        .await
        .map_err(storage_error)
    }
}

/// The `WHERE` clause `update_by_key_filter` and `delete_by_key_filter` share: only the keys sent
/// filter, folded with `AND`. Refuses an empty filter, so neither statement ever runs without one —
/// an unkeyed update or delete would touch every row.
#[allow(clippy::type_complexity)]
fn key_predicate(
    filter: MerchantThresholdsKeyFilter,
) -> StorageResult<
    Box<dyn BoxableExpression<merchant_thresholds::table, Pg, SqlType = Nullable<Bool>>>,
> {
    let mut predicates: Vec<
        Box<dyn BoxableExpression<merchant_thresholds::table, Pg, SqlType = Nullable<Bool>>>,
    > = Vec::new();

    if let Some(name) = filter.name {
        predicates.push(Box::new(dsl::name.eq(name).nullable()));
    }
    if let Some(product) = filter.product {
        predicates.push(Box::new(dsl::product.eq(product).nullable()));
    }
    if let Some(merchant_id) = filter.merchant_id {
        predicates.push(Box::new(dsl::merchant_id.eq(merchant_id).nullable()));
    }
    if let Some(profile_id) = filter.profile_id {
        predicates.push(Box::new(dsl::profile_id.eq(profile_id).nullable()));
    }

    predicates
        .into_iter()
        .reduce(|left, right| Box::new(left.and(right)))
        .ok_or_else(|| {
            report!(DatabaseError::QueryGenerationFailed).attach_printable(
                "merchant_thresholds update or delete needs at least one of name, product, \
                 merchant_id or profile_id",
            )
        })
}

fn storage_error(error: diesel::result::Error) -> error_stack::Report<DatabaseError> {
    let context = match &error {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        ) => DatabaseError::UniqueViolation,
        diesel::result::Error::NotFound => DatabaseError::NotFound,
        diesel::result::Error::QueryBuilderError(_) => DatabaseError::NoFieldsToUpdate,
        _ => DatabaseError::Others,
    };

    report!(error).change_context(context)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn key_predicate_refuses_an_empty_filter() {
        assert!(key_predicate(MerchantThresholdsKeyFilter::default()).is_err());
        assert!(key_predicate(MerchantThresholdsKeyFilter {
            name: Some("Volume Drop".to_owned()),
            ..Default::default()
        })
        .is_ok());
    }
}
