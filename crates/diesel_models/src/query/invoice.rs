use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
use crate::{
    invoice::{Invoice, InvoiceNew, InvoiceUpdate},
    schema::invoice::dsl,
    PgPooledConn, StorageResult,
};

impl InvoiceNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Invoice> {
        generics::generic_insert(conn, self).await
    }
}

impl Invoice {
    pub async fn find_invoice_by_id_invoice_id(
        conn: &PgPooledConn,
        id: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
        )
        .await
    }

    pub async fn update_invoice_entry(
        conn: &PgPooledConn,
        id: String,
        invoice_update: InvoiceUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(conn, dsl::id.eq(id.to_owned()), invoice_update)
        .await
    }

    pub async fn list_invoices_by_subscription_id(
        conn: &PgPooledConn,
        subscription_id: String,
        limit: Option<i64>,
        offset: Option<i64>,
        order_by_ascending_order: bool,
    ) -> StorageResult<Vec<Self>> {
        if order_by_ascending_order {
            generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
                conn,
                dsl::subscription_id.eq(subscription_id.to_owned()),
                limit,
                offset,
                Some(dsl::created_at.asc()),
            )
            .await
        } else {
            generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
                conn,
                dsl::subscription_id.eq(subscription_id.to_owned()),
                limit,
                offset,
                Some(dsl::created_at.desc()),
            )
            .await
        }
    }

    pub async fn get_invoice_by_subscription_id_connector_invoice_id(
        conn: &PgPooledConn,
        subscription_id: String,
        connector_invoice_id: common_utils::id_type::InvoiceId,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::subscription_id
                .eq(subscription_id.to_owned())
                .and(dsl::connector_invoice_id.eq(connector_invoice_id.to_owned())),
        )
        .await
    }
}
