use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::IntoReport;
use router_env::tracing::{self, instrument};

use super::generics::{self, ExecuteQuery, RawQuery, RawSqlQuery};
use crate::{
    enums, errors,
    payment_attempt::{
        PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate, PaymentAttemptUpdateInternal,
    },
    schema::payment_attempt::dsl,
    CustomResult, PgPooledConn,
};

impl PaymentAttemptNew {
    #[instrument(skip(conn))]
    pub async fn insert(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<PaymentAttempt, errors::DatabaseError> {
        generics::generic_insert::<_, _, PaymentAttempt, _>(conn, self, ExecuteQuery::new()).await
    }

    #[instrument(skip(conn))]
    pub async fn insert_query(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<RawSqlQuery, errors::DatabaseError> {
        generics::generic_insert::<_, _, PaymentAttempt, _>(conn, self, RawQuery).await
    }
}

impl PaymentAttempt {
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        payment_attempt: PaymentAttemptUpdate,
    ) -> CustomResult<Self, errors::DatabaseError> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            self.id,
            PaymentAttemptUpdateInternal::from(payment_attempt),
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

    #[instrument(skip(conn))]
    pub async fn update_query(
        self,
        conn: &PgPooledConn,
        payment_attempt: PaymentAttemptUpdate,
    ) -> CustomResult<RawSqlQuery, errors::DatabaseError> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            self.id,
            PaymentAttemptUpdateInternal::from(payment_attempt),
            RawQuery,
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_optional_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Option<Self>, errors::DatabaseError> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_transaction_id_payment_id_merchant_id(
        conn: &PgPooledConn,
        transaction_id: &str,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::connector_transaction_id
                .eq(transaction_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned()))
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }

    pub async fn find_last_successful_attempt_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
        // perform ordering on the application level instead of database level
        generics::generic_filter::<<Self as HasTable>::Table, _, Self>(
            conn,
            dsl::payment_id
                .eq(payment_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                .and(dsl::status.eq(enums::AttemptStatus::Charged)),
            None,
        )
        .await?
        .into_iter()
        .fold(
            Err(errors::DatabaseError::NotFound).into_report(),
            |acc, cur| match acc {
                Ok(value) if value.created_at > cur.created_at => Ok(value),
                _ => Ok(cur),
            },
        )
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_connector_txn_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        connector_txn_id: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_transaction_id.eq(connector_txn_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_transaction_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        txn_id: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::txn_id.eq(txn_id.to_owned())),
        )
        .await
    }
}
