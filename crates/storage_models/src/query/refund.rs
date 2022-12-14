use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{tracing, tracing::instrument};

use super::generics::{self, ExecuteQuery, RawQuery, RawSqlQuery};
use crate::{
    errors,
    refund::{Refund, RefundNew, RefundUpdate, RefundUpdateInternal},
    schema::refund::dsl,
    CustomResult, PgPooledConn,
};

// FIXME: Find by partition key

impl RefundNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> CustomResult<Refund, errors::DatabaseError> {
        generics::generic_insert::<_, _, Refund, _>(conn, self, ExecuteQuery::new()).await
    }

    #[instrument(skip(conn))]
    pub async fn insert_diesel_query(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<RawSqlQuery, errors::DatabaseError> {
        generics::generic_insert::<_, _, Refund, _>(conn, self, RawQuery).await
    }
}

impl Refund {
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        refund: RefundUpdate,
    ) -> CustomResult<Self, errors::DatabaseError> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            self.id,
            RefundUpdateInternal::from(refund),
            ExecuteQuery::new(),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            result => result,
        }
    }

    pub async fn update_query(
        self,
        conn: &PgPooledConn,
        refund: RefundUpdate,
    ) -> CustomResult<RawSqlQuery, errors::DatabaseError> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            self.id,
            RefundUpdateInternal::from(refund),
            RawQuery,
        )
        .await
    }

    // This is required to be changed for KV.
    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_refund_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_id: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
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
    ) -> CustomResult<Self, errors::DatabaseError> {
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
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
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
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
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

#[cfg(feature = "kv_store")]
impl crate::utils::storage_partitioning::KvStorePartition for Refund {}
