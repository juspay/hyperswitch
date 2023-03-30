use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};
use error_stack::IntoReport;
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    enums, errors,
    payment_attempt::{
        PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate, PaymentAttemptUpdateInternal,
    },
    schema::payment_attempt::dsl,
    PgPooledConn, StorageResult,
};

impl PaymentAttemptNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PaymentAttempt> {
        generics::generic_insert(conn, self).await
    }
}

impl PaymentAttempt {
    #[instrument(skip(conn))]
    pub async fn update_with_attempt_id(
        self,
        conn: &PgPooledConn,
        payment_attempt: PaymentAttemptUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::attempt_id
                .eq(self.attempt_id.to_owned())
                .and(dsl::merchant_id.eq(self.merchant_id.to_owned())),
            PaymentAttemptUpdateInternal::from(payment_attempt),
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

    #[instrument(skip(conn))]
    pub async fn find_optional_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_connector_transaction_id_payment_id_merchant_id(
        conn: &PgPooledConn,
        connector_transaction_id: &str,
        payment_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::connector_transaction_id
                .eq(connector_transaction_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned()))
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }

    pub async fn find_last_successful_attempt_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        // perform ordering on the application level instead of database level
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            Self,
        >(
            conn,
            dsl::payment_id
                .eq(payment_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                .and(dsl::status.eq(enums::AttemptStatus::Charged)),
            None,
            None,
            None,
        )
        .await?
        .into_iter()
        .fold(
            Err(errors::DatabaseError::NotFound).into_report(),
            |acc, cur| match acc {
                Ok(value) if value.modified_at > cur.modified_at => Ok(value),
                _ => Ok(cur),
            },
        )
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_connector_txn_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        connector_txn_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_transaction_id.eq(connector_txn_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_attempt_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        attempt_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::attempt_id.eq(attempt_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_payment_id_merchant_id_attempt_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
        attempt_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_id.eq(payment_id.to_owned()).and(
                dsl::merchant_id
                    .eq(merchant_id.to_owned())
                    .and(dsl::attempt_id.eq(attempt_id.to_owned())),
            ),
        )
        .await
    }
}
