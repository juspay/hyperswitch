use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{tracing, tracing::instrument};

use super::generics;
use crate::{
    connection::PgPooledConn,
    core::errors::{self, CustomResult},
    schema::customers::dsl,
    types::storage::{Customer, CustomerNew, CustomerUpdate, CustomerUpdateInternal},
};

impl CustomerNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> CustomResult<Customer, errors::StorageError> {
        generics::generic_insert::<<Customer as HasTable>::Table, _, _>(conn, self).await
    }
}

impl Customer {
    #[instrument(skip(conn))]
    pub async fn update_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: String,
        merchant_id: String,
        customer: CustomerUpdate,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (customer_id, merchant_id),
            CustomerUpdateInternal::from(customer),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn delete_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::customer_id
                .eq(customer_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
            conn,
            (customer_id.to_owned(), merchant_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_optional_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Option<Self>, errors::StorageError> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            (customer_id.to_owned(), merchant_id.to_owned()),
        )
        .await
    }
}
