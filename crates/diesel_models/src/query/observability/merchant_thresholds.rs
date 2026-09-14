use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::ReportSwitchExt;
use diesel::{associations::HasTable, ExpressionMethods};
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

    pub async fn list(conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::name.ne_all(vec![""]),
            None,
            None,
            Some((dsl::product.asc(), dsl::name.asc(), dsl::merchant_id.asc())),
        )
        .await
    }

    pub async fn update_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: uuid::Uuid,
        update: MerchantThresholdUpdate,
    ) -> StorageResult<Self> {
        let query = diesel::update(<Self as HasTable>::table())
            .filter(dsl::id.eq(id))
            .set(MerchantThresholdUpdateInternal::from(update));

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
