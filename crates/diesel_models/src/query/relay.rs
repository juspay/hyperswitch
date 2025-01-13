use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
use crate::{
    errors,
    relay::{Relay, RelayNew, RelayUpdateInternal},
    schema::relay::dsl,
    PgPooledConn, StorageResult,
};

impl RelayNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Relay> {
        generics::generic_insert(conn, self).await
    }
}

impl Relay {
    pub async fn update(
        self,
        conn: &PgPooledConn,
        relay: RelayUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(conn, dsl::id.eq(self.id.to_owned()), relay)
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            result => result,
        }
    }

    pub async fn find_by_id(
        conn: &PgPooledConn,
        id: &common_utils::id_type::RelayId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
        )
        .await
    }

    pub async fn find_by_profile_id_connector_reference_id(
        conn: &PgPooledConn,
        profile_id: &common_utils::id_type::ProfileId,
        connector_reference_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::profile_id
                .eq(profile_id.to_owned())
                .and(dsl::connector_reference_id.eq(connector_reference_id.to_owned())),
        )
        .await
    }
}
