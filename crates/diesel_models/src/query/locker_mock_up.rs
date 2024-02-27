use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    locker_mock_up::{LockerMockUp, LockerMockUpNew},
    schema::locker_mock_up::dsl,
    PgPooledConn, StorageResult,
};

impl LockerMockUpNew {
    #[instrument(skip(conn))]
    pub async fn insert_locker_mock_up(self, conn: &PgPooledConn) -> StorageResult<LockerMockUp> {
        generics::generic_insert(conn, self).await
    }
}

impl LockerMockUp {
    #[instrument(skip(conn))]
    pub async fn find_locker_by_card_id(conn: &PgPooledConn, card_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::card_id.eq(card_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn delete_locker_by_card_id(
        conn: &PgPooledConn,
        card_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::card_id.eq(card_id.to_owned()),
        )
        .await
    }
}
