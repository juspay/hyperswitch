use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::tracing::{self, instrument};

use super::generics::{self, ExecuteQuery, RawQuery, RawSqlQuery};
use crate::{
    errors,
    payment_intent::{
        PaymentIntent, PaymentIntentNew, PaymentIntentUpdate, PaymentIntentUpdateInternal,
    },
    schema::payment_intent::dsl,
    CustomResult, PgPooledConn,
};

impl PaymentIntentNew {
    #[instrument(skip(conn))]
    pub async fn insert(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<PaymentIntent, errors::DatabaseError> {
        generics::generic_insert::<_, _, PaymentIntent, _>(conn, self, ExecuteQuery::new()).await
    }

    #[instrument(skip(conn))]
    pub async fn insert_query(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<RawSqlQuery, errors::DatabaseError> {
        generics::generic_insert::<_, _, PaymentIntent, _>(conn, self, RawQuery).await
    }
}

impl PaymentIntent {
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        payment_intent: PaymentIntentUpdate,
    ) -> CustomResult<Self, errors::DatabaseError> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            self.id,
            PaymentIntentUpdateInternal::from(payment_intent),
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
        payment_intent: PaymentIntentUpdate,
    ) -> CustomResult<RawSqlQuery, errors::DatabaseError> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            self.id,
            PaymentIntentUpdateInternal::from(payment_intent),
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
}
