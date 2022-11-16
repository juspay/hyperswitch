use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{tracing, tracing::instrument};

use super::generics;
use crate::{
    connection::PgPooledConn,
    core::errors::{self, CustomResult, DatabaseError},
    schema::address::dsl,
    types::storage::{Address, AddressNew, AddressUpdate, AddressUpdateInternal},
};

impl AddressNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> CustomResult<Address, errors::StorageError> {
        generics::generic_insert::<<Address as HasTable>::Table, _, _>(conn, self).await
    }
}

impl Address {
    #[instrument(skip(conn))]
    pub async fn update_by_address_id(
        conn: &PgPooledConn,
        address_id: String,
        address: AddressUpdate,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            address_id,
            AddressUpdateInternal::from(address),
        )
        .await
        .map_err(|error| match error.current_context() {
            errors::StorageError::DatabaseError(DatabaseError::NotFound) => {
                error.attach_printable("Address with the given ID doesn't exist")
            }
            _ => error,
        })
    }

    #[instrument(skip(conn))]
    pub async fn delete_by_address_id(
        conn: &PgPooledConn,
        address_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::address_id.eq(address_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_address_id<'a>(
        conn: &PgPooledConn,
        address_id: &str,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, address_id.to_owned())
            .await
    }

    #[instrument(skip(conn))]
    pub async fn find_optional_by_address_id<'a>(
        conn: &PgPooledConn,
        address_id: &str,
    ) -> CustomResult<Option<Self>, errors::StorageError> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            address_id.to_owned(),
        )
        .await
    }
}
