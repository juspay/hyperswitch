use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::report;

use super::generics;
use crate::{
    dynamic_routing_stats::{
        DynamicRoutingStats, DynamicRoutingStatsNew, DynamicRoutingStatsUpdate,
    },
    errors,
    schema::dynamic_routing_stats::dsl,
    PgPooledConn, StorageResult,
};

impl DynamicRoutingStatsNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<DynamicRoutingStats> {
        generics::generic_insert(conn, self).await
    }
}

impl DynamicRoutingStats {
    pub async fn find_optional_by_attempt_id_merchant_id(
        conn: &PgPooledConn,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::attempt_id.eq(attempt_id.to_owned())),
        )
        .await
    }

    pub async fn update(
        conn: &PgPooledConn,
        attempt_id: String,
        merchant_id: &common_utils::id_type::MerchantId,
        dynamic_routing_stat: DynamicRoutingStatsUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<
            <Self as HasTable>::Table,
            DynamicRoutingStatsUpdate,
            _,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::attempt_id.eq(attempt_id.to_owned())),
            dynamic_routing_stat,
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound)
                .attach_printable("Error while updating dynamic_routing_stats entry")
        })
    }
}
