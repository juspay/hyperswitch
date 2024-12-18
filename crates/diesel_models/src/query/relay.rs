use super::generics;
use crate::{
    errors,
    relay::{Relay, RelayNew, RelayUpdate, RelayUpdateInternal},
    schema::relay::dsl,
    PgPooledConn, StorageResult,
};
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

impl RelayNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Relay> {
        generics::generic_insert(conn, self).await
    }
}

impl Relay {
    pub async fn update(self, conn: &PgPooledConn, relay: RelayUpdate) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::id
                .eq(self.id.to_owned())
                .and(dsl::merchant_id.eq(self.merchant_id.to_owned())),
            RelayUpdateInternal::from(relay),
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

    pub async fn find_by_id(
        conn: &PgPooledConn,
        id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
        )
        .await
    }
}
