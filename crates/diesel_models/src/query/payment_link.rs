use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
// use error_stack::report;
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    // errors,
    payment_link::{PaymentLink, PaymentLinkNew},
    schema::payment_link::dsl,
    PgPooledConn, StorageResult,
};

impl PaymentLinkNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PaymentLink> {
        generics::generic_insert(conn, self).await
    }
}

impl PaymentLink {
    pub async fn find_by_payment_id(
        conn: &PgPooledConn,
        payment_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_id
                .eq(payment_id.to_owned())
        )
        .await
    }
}