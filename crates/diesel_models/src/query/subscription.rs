use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::report;

use super::generics;
use crate::{
    errors,
    schema::subscription::dsl,
    subscription::{Subscription, SubscriptionNew, SubscriptionUpdate},
    PgPooledConn, StorageResult,
};

impl SubscriptionNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Subscription> {
        generics::generic_insert(conn, self).await
    }
}

impl Subscription {
    pub async fn find_by_merchant_id_subscription_id(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        id: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::id.eq(id.to_owned())),
        )
        .await
    }

    pub async fn update_subscription_entry(
        conn: &PgPooledConn,
        merchant_id: &common_utils::id_type::MerchantId,
        id: String,
        subscription_update: SubscriptionUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<
            <Self as HasTable>::Table,
            SubscriptionUpdate,
            _,
            _,
        >(
            conn,
            dsl::id
                .eq(id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
            subscription_update,
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound)
                .attach_printable("Error while updating subscription entry")
        })
    }
}
