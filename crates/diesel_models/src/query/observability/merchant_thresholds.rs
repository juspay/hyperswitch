use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::ReportSwitchExt;
use diesel::{
    associations::HasTable,
    dsl::sql,
    expression::SqlLiteral,
    sql_types::{Jsonb, Nullable},
    ExpressionMethods, PgJsonbExpressionMethods, QueryDsl,
};
use error_stack::ResultExt;

use crate::{
    observability::{
        merchant_thresholds::{
            MerchantThreshold, MerchantThresholdNew, MerchantThresholdUpdate,
            MerchantThresholdUpdateInternal,
        },
        schema::merchant_thresholds::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

impl MerchantThresholdNew {
    pub async fn upsert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
        update: MerchantThresholdUpdate,
    ) -> StorageResult<MerchantThreshold> {
        let query = diesel::insert_into(<MerchantThreshold as HasTable>::table())
            .values(self)
            .on_conflict((
                dsl::name,
                dsl::product,
                dsl::merchant_id,
                dsl::is_enabled,
                dsl::author,
            ))
            .do_update()
            .set(MerchantThresholdUpdateInternal::from(update));

        generics::db_metrics::track_database_call::<<MerchantThreshold as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Insert,
            query.get_result_async(conn.raw_connection()),
        )
        .await
        .attach_printable("Failed to upsert the merchant threshold")
        .switch()
    }
}

impl MerchantThreshold {
    pub async fn find_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: uuid::Uuid,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, id).await
    }

    pub async fn filter_by_constraints(
        conn: &DatabaseConnectionWithContext<'_>,
        name: Option<String>,
        product: Option<String>,
        merchant_id: Option<String>,
        is_enabled: Option<bool>,
        author: Option<String>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = crate::list::into_boxed_list(<Self as HasTable>::table().order((
            dsl::product.asc(),
            dsl::name.asc(),
            dsl::merchant_id.asc(),
        )));

        if let Some(name) = name {
            query = query.filter(dsl::name.eq(name));
        }
        if let Some(product) = product {
            query = query.filter(dsl::product.eq(product));
        }
        if let Some(merchant_id) = merchant_id {
            query = query.filter(dsl::merchant_id.eq(merchant_id));
        }
        if let Some(is_enabled) = is_enabled {
            query = query.filter(dsl::is_enabled.eq(is_enabled));
        }
        if let Some(author) = author {
            query = query.filter(dsl::author.eq(author));
        }

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Filter,
            query.get_results_async(conn.raw_connection()),
        )
        .await
        .attach_printable("Failed to filter the merchant thresholds by constraints")
        .switch()
    }

    pub async fn update_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: uuid::Uuid,
        update: MerchantThresholdUpdate,
    ) -> StorageResult<Self> {
        let mut changeset = MerchantThresholdUpdateInternal::from(update);
        let metadata = changeset
            .metadata
            .take()
            .map(|patch| dsl::metadata.eq(stored_metadata().concat(patch)));

        let query = diesel::update(<Self as HasTable>::table())
            .filter(dsl::id.eq(id))
            .set((changeset, metadata));

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::UpdateOne,
            query.get_result_async(conn.raw_connection()),
        )
        .await
        .attach_printable("Failed to update the merchant threshold")
        .switch()
    }

    pub async fn delete_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: uuid::Uuid,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(conn, dsl::id.eq(id)).await
    }
}

fn stored_metadata() -> SqlLiteral<Nullable<Jsonb>> {
    sql::<Nullable<Jsonb>>("COALESCE(metadata, '{}'::jsonb)")
}
