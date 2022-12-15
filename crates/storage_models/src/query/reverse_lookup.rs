use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{tracing, tracing::instrument};

use super::generics::{self, ExecuteQuery};
use crate::{
    errors,
    reverse_lookup::{ReverseLookup, ReverseLookupNew},
    schema::reverse_lookup::dsl,
    CustomResult, PgPooledConn,
};

impl ReverseLookupNew {
    #[instrument(skip(conn))]
    pub async fn insert(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<ReverseLookup, errors::DatabaseError> {
        generics::generic_insert::<_, _, ReverseLookup, _>(conn, self, ExecuteQuery::new()).await
    }
}
impl ReverseLookup {
    pub async fn find_by_lookup_id(
        lookup_id: &str,
        conn: &PgPooledConn,
    ) -> CustomResult<Self, errors::DatabaseError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::lookup_id.eq(lookup_id.to_owned()),
        )
        .await
    }
}
