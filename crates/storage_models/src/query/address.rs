use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    address::{Address, AddressNew, AddressUpdate, AddressUpdateInternal},
    errors,
    schema::address::dsl,
    PgPooledConn, StorageResult,
};

impl AddressNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Address> {
        generics::generic_insert(conn, self).await
    }
}

impl Address {
    #[instrument(skip(conn))]
    pub async fn update_by_address_id(
        conn: &PgPooledConn,
        address_id: String,
        address: AddressUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            address_id.clone(),
            AddressUpdateInternal::from(address),
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
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::address_id.eq(address_id.to_owned()),
        )
        .await
    }

    pub async fn update_by_merchant_id_customer_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
        address: AddressUpdate,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::customer_id.eq(customer_id.to_owned())),
            AddressUpdateInternal::from(address),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_address_id<'a>(
        conn: &PgPooledConn,
        address_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, address_id.to_owned())
            .await
    }

    #[instrument(skip(conn))]
    pub async fn find_optional_by_address_id<'a>(
        conn: &PgPooledConn,
        address_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            address_id.to_owned(),
        )
        .await
    }
}
