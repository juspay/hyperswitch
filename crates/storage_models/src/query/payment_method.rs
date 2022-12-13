use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::ResultExt;
use router_env::tracing::{self, instrument};

use super::generics::{self, ExecuteQuery};
use crate::{
    errors,
    payment_method::{PaymentMethod, PaymentMethodNew},
    schema::payment_methods::dsl,
    CustomResult, PgPooledConn,
};

impl PaymentMethodNew {
    #[instrument(skip(conn))]
    pub async fn insert(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<PaymentMethod, errors::DatabaseError> {
        generics::generic_insert::<_, _, PaymentMethod, _>(conn, self, ExecuteQuery::new()).await
    }
}

impl PaymentMethod {
    #[instrument(skip(conn))]
    pub async fn delete_by_payment_method_id(
        conn: &PgPooledConn,
        payment_method_id: String,
    ) -> CustomResult<Self, errors::DatabaseError> {
        let result =
            generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, Self, _>(
                conn,
                dsl::payment_method_id.eq(payment_method_id),
                ExecuteQuery::new(),
            )
            .await
            .attach_printable("Error while deleting by payment method ID")?;
        Ok(result)
    }

    #[instrument(skip(conn))]
    pub async fn delete_by_merchant_id_payment_method_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
        let result =
            generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, Self, _>(
                conn,
                dsl::merchant_id
                    .eq(merchant_id.to_owned())
                    .and(dsl::payment_method_id.eq(payment_method_id.to_owned())),
                ExecuteQuery::new(),
            )
            .await?;

        Ok(result)
    }

    #[instrument(skip(conn))]
    pub async fn find_by_payment_method_id(
        conn: &PgPooledConn,
        payment_method_id: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_method_id.eq(payment_method_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            None,
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::customer_id
                .eq(customer_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
            None,
        )
        .await
    }
}
