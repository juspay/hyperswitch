use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, debug_query, pg::Pg, BoolExpressionMethods, ExpressionMethods, QueryDsl};
use error_stack::ResultExt;

use super::generics;
use crate::{
    errors,
    schema::subscription::dsl,
    subscription::{Subscription, SubscriptionNew},
    PgPooledConn, StorageResult,
};

impl SubscriptionNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Subscription> {
        generics::generic_insert(conn, self).await
    }
}

impl Subscription {
    pub async fn find_by_merchant_id_customer_id_subscription_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        customer_id: &common_utils::id_type::CustomerId,
        subscription_id: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::subscription_id.eq(subscription_id.to_owned()))
                .and(dsl::customer_id.eq(customer_id.to_owned())),
        )
        .await
    }

    pub async fn find_payment_method_ids_by_billing_connector_subscription_id(
        conn: &PgPooledConn,
        subscription_id: &str,
    ) -> StorageResult<Vec<String>> {
        let query = <Self as HasTable>::table()
            .select(dsl::payment_method_id)
            .filter(dsl::subscription_id.eq(subscription_id.to_owned()))
            .filter(dsl::payment_method_id.is_not_null())
            .order(dsl::created_at.desc())
            .into_boxed();

        router_env::logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            query.load_async::<Option<String>>(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to find payment method IDs by billing connector subscription ID")
        .map(|results| results.into_iter().filter_map(|x| x).collect())
    }
}
