use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable,
    query_dsl::methods::{BoxedDsl, LimitDsl, OffsetDsl, OrderDsl},
    ExpressionMethods,
};
use error_stack::ResultExt;

use crate::{
    errors, hyperswitch_ai_interaction::*, query::generics,
    schema::hyperswitch_ai_interaction::dsl, PgPooledConn, StorageResult,
};
use diesel::query_dsl::QueryDsl;

impl HyperswitchAiInteractionNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<HyperswitchAiInteraction> {
        generics::generic_insert(conn, self).await
    }
}

impl HyperswitchAiInteraction {
    pub async fn filter_by_optional_merchant_id(
        conn: &PgPooledConn,
        merchant_id: Option<&common_utils::id_type::MerchantId>,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<Self>> {
        let mut query = Self::table().into_boxed();
        // if let Some(merchant_id) = merchant_id {
        //     query = query.filter(dsl::merchant_id.eq(merchant_id));
        // }
        // query = query
        //     .limit(limit)
        //     .offset(offset)
        //     .order_by(dsl::created_at.desc());
        generics::db_metrics::track_database_call::<Self, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error filtering hyperswitch_ai_interaction by merchant_id")
    }
}
