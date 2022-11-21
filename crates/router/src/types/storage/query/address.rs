use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{tracing, tracing::instrument};

use super::generics::{self, ExecuteQuery};
use crate::{
    connection::PgPooledConn,
    core::errors::{self, CustomResult, DatabaseError},
    schema::address::dsl,
    types::storage::{Address, AddressNew, AddressUpdate, AddressUpdateInternal},
};

impl AddressNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> CustomResult<Address, errors::StorageError> {
        generics::generic_insert::<_, _, Address, _>(conn, self, ExecuteQuery::new()).await
    }
}

impl Address {
    #[instrument(skip(conn))]
    pub async fn update_by_address_id(
        conn: &PgPooledConn,
        address_id: String,
        address: AddressUpdate,
    ) -> CustomResult<Self, errors::StorageError> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            address_id.clone(),
            AddressUpdateInternal::from(address),
            ExecuteQuery::new(),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::StorageError::DatabaseError(DatabaseError::NotFound) => {
                    Err(error.attach_printable("Address with the given ID doesn't exist"))
                }
                errors::StorageError::DatabaseError(errors::DatabaseError::NoFieldsToUpdate) => {
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
    ) -> CustomResult<bool, errors::StorageError> {
        generics::generic_delete::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::address_id.eq(address_id.to_owned()),
            ExecuteQuery::<Self>::new(),
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
