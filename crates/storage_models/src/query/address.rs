use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{tracing, tracing::instrument};

use super::generics::{self, ExecuteQuery};
use crate::{
    address::{Address, AddressNew, AddressUpdate, AddressUpdateInternal},
    errors,
    schema::address::dsl,
    CustomResult, PgPooledConn,
};

impl AddressNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> CustomResult<Address, errors::DatabaseError> {
        generics::generic_insert::<_, _, Address, _>(conn, self, ExecuteQuery::new()).await
    }
}

impl Address {
    #[instrument(skip(conn))]
    pub async fn update_by_address_id(
        conn: &PgPooledConn,
        address_id: String,
        address: AddressUpdate,
    ) -> CustomResult<Self, errors::DatabaseError> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            address_id.clone(),
            AddressUpdateInternal::from(address),
            ExecuteQuery::new(),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NotFound => {
                    Err(error.attach_printable("Address with the given ID doesn't exist"))
                }
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
                        conn,
                        address_id.clone(),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    #[instrument(skip(conn))]
    pub async fn delete_by_address_id(
        conn: &PgPooledConn,
        address_id: &str,
    ) -> CustomResult<bool, errors::DatabaseError> {
        generics::generic_delete::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::address_id.eq(address_id.to_owned()),
            ExecuteQuery::<Self>::new(),
        )
        .await
    }

    pub async fn update_by_merchant_id_customer_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
        address: AddressUpdate,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::customer_id.eq(customer_id.to_owned())),
            AddressUpdateInternal::from(address),
            ExecuteQuery::new(),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_address_id<'a>(
        conn: &PgPooledConn,
        address_id: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, address_id.to_owned())
            .await
    }

    #[instrument(skip(conn))]
    pub async fn find_optional_by_address_id<'a>(
        conn: &PgPooledConn,
        address_id: &str,
    ) -> CustomResult<Option<Self>, errors::DatabaseError> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            address_id.to_owned(),
        )
        .await
    }
}
