use diesel::{associations::HasTable, ExpressionMethods};

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
}
