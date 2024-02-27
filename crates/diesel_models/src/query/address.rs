use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
use crate::{
    address::{Address, AddressNew, AddressUpdateInternal},
    errors,
    schema::address::dsl,
    PgPooledConn, StorageResult,
};

impl AddressNew {
    pub async fn insert_address(self, conn: &PgPooledConn) -> StorageResult<Address> {
        generics::generic_insert(conn, self).await
    }
}

impl Address {
    pub async fn find_by_address_id<'a>(
        conn: &PgPooledConn,
        address_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, address_id.to_owned())
            .await
    }

    pub async fn update_by_address_id(
        conn: &PgPooledConn,
        address_id: String,
        address: AddressUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            address_id.clone(),
            address,
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

    pub async fn update_address(
        self,
        conn: &PgPooledConn,
        address_update_internal: AddressUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::address_id.eq(self.address_id.clone()),
            address_update_internal,
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
        address: AddressUpdateInternal,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::customer_id.eq(customer_id.to_owned())),
            address,
        )
        .await
    }

    pub async fn find_by_merchant_id_payment_id_address_id<'a>(
        conn: &PgPooledConn,
        merchant_id: &str,
        payment_id: &str,
        address_id: &str,
    ) -> StorageResult<Self> {
        match generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_id
                .eq(payment_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                .and(dsl::address_id.eq(address_id.to_owned())),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NotFound => {
                    generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
                        conn,
                        address_id.to_owned(),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

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
