use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{tracing, tracing::instrument};

use super::generics::{self, ExecuteQuery};
use crate::{
    errors,
    reverse_lookup::{
        ReverseLookup, ReverseLookupNew, ReverseLookupUpdate, ReverseLookupUpdateInternal,
    },
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
    #[instrument(skip(conn))]
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
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        lookup: ReverseLookupUpdate,
    ) -> CustomResult<Self, errors::DatabaseError> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            self.lookup_id.clone(),
            ReverseLookupUpdateInternal::from(lookup),
            ExecuteQuery::new(),
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
}
