use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{tracing, tracing::instrument};

use super::generics;
use crate::{
    connection::PgPooledConn,
    core::errors::{self, CustomResult},
    schema::refund::dsl,
    types::storage::{Refund, RefundNew, RefundUpdate, RefundUpdateInternal},
};

// FIXME: Find by partition key

impl RefundNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> CustomResult<Refund, errors::StorageError> {
        generics::generic_insert::<<Refund as HasTable>::Table, _, _>(conn, self).await
    }
}

impl Refund {
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        refund: RefundUpdate,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.id,
            RefundUpdateInternal::from(refund),
        )
        .await
    }

    // This is required to be changed for KV.
    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_refund_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_id: &str,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::refund_id.eq(refund_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_internal_reference_id_merchant_id(
        conn: &PgPooledConn,
        internal_reference_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::internal_reference_id.eq(internal_reference_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_transaction_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        txn_id: &str,
    ) -> CustomResult<Vec<Self>, errors::StorageError> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::transaction_id.eq(txn_id.to_owned())),
            None,
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<Self>, errors::StorageError> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
            None,
        )
        .await
    }
}
