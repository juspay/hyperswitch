use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::tracing::{self, instrument};

use crate::{
    errors, fraud_check::*, query::generics, schema::fraud_check::dsl, PgPooledConn, StorageResult,
};

impl FraudCheckNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<FraudCheck> {
        generics::generic_insert(conn, self).await
    }
}

impl FraudCheck {
    pub async fn update_with_attempt_id(
        self,
        conn: &PgPooledConn,
        fraud_check: FraudCheckUpdate,
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
            FraudCheckUpdateInternal::from(fraud_check),
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

    pub async fn get_with_payment_id(
        conn: &PgPooledConn,
        payment_id: String,
        merchant_id: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_id
                .eq(payment_id)
                .and(dsl::merchant_id.eq(merchant_id)),
        )
        .await
    }

    pub async fn get_with_payment_id_if_present(
        conn: &PgPooledConn,
        payment_id: String,
        merchant_id: String,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_id
                .eq(payment_id)
                .and(dsl::merchant_id.eq(merchant_id)),
        )
        .await
    }
}
