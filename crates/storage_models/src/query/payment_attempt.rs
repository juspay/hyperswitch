use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, debug_query, pg::Pg, BoolExpressionMethods, ExpressionMethods, QueryDsl,
};
use error_stack::{IntoReport, ResultExt};
use router_env::{
    logger::debug,
    tracing::{self, instrument},
};

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

    // FIXME: Use generics
    #[instrument(skip(conn))]
    pub async fn find_last_successful_attempt_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
        let query = Self::table()
            .filter(
                dsl::payment_id
                    .eq(payment_id.to_owned())
                    .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                    .and(dsl::status.eq(enums::AttemptStatus::Charged)),
            )
            .order(dsl::created_at.desc());
        debug!(query = %debug_query::<Pg, _>(&query).to_string());

        query
            .get_result_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::NotFound)
            .attach_printable("Error while finding last successful payment attempt")
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
