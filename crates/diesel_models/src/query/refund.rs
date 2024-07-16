use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};

use super::generics;
use crate::{
    errors,
    refund::{Refund, RefundNew, RefundUpdate, RefundUpdateInternal},
    schema::refund::dsl,
    PgPooledConn, StorageResult,
};

impl RefundNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Refund> {
        generics::generic_insert(conn, self).await
    }
}

impl Refund {
    pub async fn update(self, conn: &PgPooledConn, refund: RefundUpdate) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::refund_id
                .eq(self.refund_id.to_owned())
                .and(dsl::merchant_id.eq(self.merchant_id.to_owned())),
            RefundUpdateInternal::from(refund),
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

    // This is required to be changed for KV.
    pub async fn find_by_merchant_id_refund_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::refund_id.eq(refund_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_merchant_id_connector_refund_id_connector(
        conn: &PgPooledConn,
        merchant_id: &str,
        connector_refund_id: &str,
        connector: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_refund_id.eq(connector_refund_id.to_owned()))
                .and(dsl::connector.eq(connector.to_owned())),
        )
        .await
    }

    pub async fn find_by_internal_reference_id_merchant_id(
        conn: &PgPooledConn,
        internal_reference_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::internal_reference_id.eq(internal_reference_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_merchant_id_connector_transaction_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        connector_transaction_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_transaction_id.eq(connector_transaction_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }

    pub async fn find_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }
}
