use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
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
    pub async fn find_by_link_payment_link_id(
        conn: &PgPooledConn,
        payment_link_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_link_id.eq(payment_link_id.to_owned()),
        )
        .await
    }
}
