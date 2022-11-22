use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{tracing, tracing::instrument};

use super::generics::{self, ExecuteQuery};
use crate::{
    connection::PgPooledConn,
    core::errors::{self, CustomResult},
    schema::locker_mock_up::dsl,
    types::storage::{LockerMockUp, LockerMockUpNew},
};

impl LockerMockUpNew {
    #[instrument(skip(conn))]
    pub async fn insert(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<LockerMockUp, errors::StorageError> {
        generics::generic_insert::<_, _, LockerMockUp, _>(conn, self, ExecuteQuery::new()).await
    }
}

impl LockerMockUp {
    #[instrument(skip(conn))]
    pub async fn find_by_card_id(
        conn: &PgPooledConn,
        card_id: &str,
    ) -> CustomResult<Self, errors::StorageError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::card_id.eq(card_id.to_owned()),
        )
        .await
    }
}
