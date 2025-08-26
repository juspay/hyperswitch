use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
use crate::{
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
}
