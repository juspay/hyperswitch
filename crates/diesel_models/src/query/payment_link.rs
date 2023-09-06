// use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
// use error_stack::report;
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    // errors,
    payment_link::{PaymentLink, PaymentLinkNew},
    // schema::payment_link::dsl,
    PgPooledConn, StorageResult,
};

impl PaymentLinkNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PaymentLink> {
        generics::generic_insert(conn, self).await
    }
}