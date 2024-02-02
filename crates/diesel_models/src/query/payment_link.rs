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
        /// Asynchronously inserts a PaymentLink struct into the database using the provided connection.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PaymentLink> {
        generics::generic_insert(conn, self).await
    }
}

impl PaymentLink {
        /// Asynchronously finds a link by the provided payment link ID in the database.
    pub async fn find_link_by_payment_link_id(
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
