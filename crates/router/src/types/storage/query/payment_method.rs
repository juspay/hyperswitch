use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::report;
use router_env::tracing::{self, instrument};

use super::generics;
use crate::{
    connection::PgPooledConn,
    core::errors::{self, CustomResult},
    schema::payment_methods::dsl,
    types::storage::payment_method::{PaymentMethod, PaymentMethodNew},
};

impl PaymentMethodNew {
    #[instrument(skip(conn))]
    pub async fn insert(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<PaymentMethod, errors::StorageError> {
        generics::generic_insert::<<PaymentMethod as HasTable>::Table, _, _>(conn, self).await
    }
}

impl PaymentMethod {
    #[instrument(skip(conn))]
    pub async fn delete_by_payment_method_id(
        conn: &PgPooledConn,
        payment_method_id: String,
    ) -> CustomResult<Self, errors::StorageError> {
        let result = generics::generic_delete_with_results::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_method_id.eq(payment_method_id),
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::StorageError::DatabaseError(
                errors::DatabaseError::NotFound
            ))
            .attach_printable("Error while deleting by payment method ID")
        })?;
        Ok(result)
    }

    #[instrument(skip(conn))]
    pub async fn delete_by_merchant_id_payment_method_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> CustomResult<Self, errors::StorageError> {
        let result = generics::generic_delete_one_with_results::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_method_id.eq(payment_method_id.to_owned())),
        )
        .await?;

        Ok(result)
    }

    #[instrument(skip(conn))]
    pub async fn find_by_payment_method_id(
        conn: &PgPooledConn,
        payment_method_id: &str,
    ) -> CustomResult<Self, errors::StorageError> {
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
    ) -> CustomResult<Vec<Self>, errors::StorageError> {
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
    ) -> CustomResult<Vec<Self>, errors::StorageError> {
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
