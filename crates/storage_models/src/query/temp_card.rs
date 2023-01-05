use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    schema::temp_card::dsl,
    temp_card::{TempCard, TempCardNew},
    PgPooledConn, StorageResult,
};

impl TempCardNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<TempCard> {
        generics::generic_insert(conn, self).await
    }
}

impl TempCard {
    #[instrument(skip(conn))]
    pub async fn insert_with_token(self, conn: &PgPooledConn) -> StorageResult<Self> {
        generics::generic_insert(conn, self).await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_transaction_id(
        conn: &PgPooledConn,
        transaction_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::txn_id.eq(transaction_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_token(conn: &PgPooledConn, token: &i32) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq(token.to_owned()),
        )
        .await
    }
}
