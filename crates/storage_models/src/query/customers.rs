use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{tracing, tracing::instrument};

use super::generics::{self, ExecuteQuery};
use crate::{
    customers::{Customer, CustomerNew, CustomerUpdate, CustomerUpdateInternal},
    errors,
    schema::customers::dsl,
    CustomResult, PgPooledConn,
};

impl CustomerNew {
    #[instrument(skip(conn))]
    pub async fn insert(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<Customer, errors::DatabaseError> {
        generics::generic_insert::<_, _, Customer, _>(conn, self, ExecuteQuery::new()).await
    }
}

impl Customer {
    #[instrument(skip(conn))]
    pub async fn update_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: String,
        merchant_id: String,
        customer: CustomerUpdate,
    ) -> CustomResult<Self, errors::DatabaseError> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            (customer_id.clone(), merchant_id.clone()),
            CustomerUpdateInternal::from(customer),
            ExecuteQuery::new(),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
                        conn,
                        (customer_id, merchant_id),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    #[instrument(skip(conn))]
    pub async fn delete_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::DatabaseError> {
        generics::generic_delete::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::customer_id
                .eq(customer_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
            ExecuteQuery::<Self>::new(),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
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
    ) -> CustomResult<Option<Self>, errors::DatabaseError> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            (customer_id.to_owned(), merchant_id.to_owned()),
        )
        .await
    }
}
