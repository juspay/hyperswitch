use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable,
    debug_query,
    expression_methods::{PgAnyJsonExpressionMethods, PgJsonbExpressionMethods},
    pg::Pg,
    BoolExpressionMethods, ExpressionMethods, QueryDsl,
};
use error_stack::ResultExt;
use router_env::logger;

use crate::{
    errors::DatabaseError,
    observability::{
        alert_manager::merchants_alert_external_config::{
            MerchantsAlertExternalConfig, MerchantsAlertExternalConfigListFilter,
            MerchantsAlertExternalConfigNew, MerchantsAlertExternalConfigUpdate,
        },
        schema::merchants_alert_external_config::dsl,
    },
    query::{
        generics,
        generics::db_metrics::{track_database_call, DatabaseOperation},
    },
    DatabaseConnectionWithContext, StorageResult,
};

impl MerchantsAlertExternalConfigNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<MerchantsAlertExternalConfig> {
        generics::generic_insert(conn, self).await
    }
}

impl MerchantsAlertExternalConfig {
    pub async fn find_by_name_product(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        product: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
            conn,
            (name.to_owned(), product.to_owned()),
        )
        .await
    }

    pub async fn list_by_filter(
        conn: &DatabaseConnectionWithContext<'_>,
        filter: MerchantsAlertExternalConfigListFilter,
    ) -> StorageResult<Vec<Self>> {
        let mut query = crate::list::into_boxed_list(
            Self::table().order((dsl::name.asc(), dsl::product.asc())),
        );

        if let Some(names) = filter.names {
            query = query.filter(dsl::name.eq_any(names));
        }

        if let Some(products) = filter.products {
            query = query.filter(dsl::product.eq_any(products));
        }

        if let Some(categories) = filter.categories {
            query = query.filter(dsl::category.eq_any(categories));
        }

        if let Some(is_enabled) = filter.is_enabled {
            query = query.filter(dsl::is_enabled.eq(is_enabled));
        }

        for (key, values) in filter.metadata {
            query = query.filter(dsl::metadata.retrieve_as_text(key).eq_any(values));
        }

        if let Some(start) = filter.last_updated_at_start {
            query = query.filter(dsl::last_updated_at.ge(start));
        }

        if let Some(end) = filter.last_updated_at_end {
            query = query.filter(dsl::last_updated_at.le(end));
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
        .attach_printable("Failed to list merchants_alert_external_config")
    }

    pub async fn update_by_name_product(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        product: &str,
        update: MerchantsAlertExternalConfigUpdate,
    ) -> StorageResult<Self> {
        let changeset = (
            update.category.map(|value| dsl::category.eq(value)),
            update.is_enabled.map(|value| dsl::is_enabled.eq(value)),
            update
                .metadata
                .map(|value| dsl::metadata.eq(dsl::metadata.concat(value))),
            dsl::last_updated_at.eq(update.last_updated_at),
        );

        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::name
                .eq(name.to_owned())
                .and(dsl::product.eq(product.to_owned())),
            changeset,
        )
        .await
    }

    pub async fn delete_by_name_product(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        product: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::name
                .eq(name.to_owned())
                .and(dsl::product.eq(product.to_owned())),
        )
        .await
    }
}
