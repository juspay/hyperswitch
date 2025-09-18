use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::report;

use super::generics;
use crate::{
    errors,
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
    pub async fn find_by_id(conn: &PgPooledConn, id: String) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
        )
        .await
    }

    pub async fn find_by_merchant_id_invoice_id(
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

    pub async fn update_invoice_entry(
        conn: &PgPooledConn,
        id: String,
        invoice_update: InvoiceUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, InvoiceUpdate, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
            invoice_update,
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound)
                .attach_printable("Error while updating invoice entry")
        })
    }
}
