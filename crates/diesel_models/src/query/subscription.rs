use diesel::{
    associations::HasTable, dsl::exists, BoolExpressionMethods, ExpressionMethods, QueryDsl,
};
use error_stack::report;

use super::generics;
use crate::{
    errors,
    schema::{invoice, subscription::dsl},
    subscription::{Subscription, SubscriptionNew, SubscriptionUpdate},
    DatabaseConnectionWithContext, StorageResult,
};

impl SubscriptionNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<Subscription> {
        generics::generic_insert(conn, self).await
    }
}

impl Subscription {
    pub async fn find_by_merchant_id_subscription_id(
        conn: &DatabaseConnectionWithContext<'_>,
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

    pub async fn find_by_merchant_id_connector_subscription_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        connector_subscription_id: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::merchant_connector_id.eq(Some(merchant_connector_id.to_owned())))
                .and(dsl::connector_subscription_id.eq(Some(connector_subscription_id))),
        )
        .await
    }

    pub async fn update_subscription_entry(
        conn: &DatabaseConnectionWithContext<'_>,
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

    pub async fn update_subscription_entry_if_status(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        id: String,
        expected_status: String,
        subscription_update: SubscriptionUpdate,
    ) -> StorageResult<Option<Self>> {
        generics::generic_update_with_results::<
            <Self as HasTable>::Table,
            SubscriptionUpdate,
            _,
            _,
        >(
            conn,
            dsl::id
                .eq(id)
                .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                .and(dsl::status.eq(expected_status)),
            subscription_update,
        )
        .await
        .map(|mut rows| rows.pop())
    }

    pub async fn update_subscription_entry_if_status_and_invoice_status(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        id: String,
        expected_status: String,
        invoice_id: common_utils::id_type::InvoiceId,
        expected_invoice_status: common_enums::InvoiceStatus,
        billing_period_end: time::PrimitiveDateTime,
        subscription_update: SubscriptionUpdate,
    ) -> StorageResult<Option<Self>> {
        let invoice_matches = exists(
            invoice::table.filter(
                invoice::id
                    .eq(invoice_id)
                    .and(invoice::subscription_id.eq(id.clone()))
                    .and(invoice::merchant_id.eq(merchant_id.to_owned()))
                    .and(invoice::status.eq(expected_invoice_status))
                    .and(invoice::billing_period_end.eq(Some(billing_period_end))),
            ),
        );
        generics::generic_update_with_results::<
            <Self as HasTable>::Table,
            SubscriptionUpdate,
            _,
            _,
        >(
            conn,
            dsl::id
                .eq(id)
                .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                .and(dsl::status.eq(expected_status))
                .and(invoice_matches)
                .and(
                    dsl::last_applied_billing_period_end
                        .is_null()
                        .or(dsl::last_applied_billing_period_end.le(Some(billing_period_end))),
                ),
            subscription_update,
        )
        .await
        .map(|mut rows| rows.pop())
    }

    pub async fn bind_connector_subscription_id_if_unset_or_equal(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        id: String,
        connector_subscription_id: String,
        subscription_update: SubscriptionUpdate,
    ) -> StorageResult<Option<Self>> {
        generics::generic_update_with_results::<
            <Self as HasTable>::Table,
            SubscriptionUpdate,
            _,
            _,
        >(
            conn,
            dsl::id
                .eq(id)
                .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                .and(
                    dsl::connector_subscription_id
                        .is_null()
                        .or(dsl::connector_subscription_id.eq(Some(connector_subscription_id))),
                ),
            subscription_update,
        )
        .await
        .map(|mut rows| rows.pop())
    }

    pub async fn list_by_merchant_id_profile_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::profile_id.eq(profile_id.to_owned())),
            limit,
            offset,
            Some(dsl::created_at.desc()),
        )
        .await
    }
}
