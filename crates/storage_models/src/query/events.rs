use router_env::tracing::{self, instrument};

use super::generics;
use crate::{
    errors,
    events::{Event, EventNew},
    CustomResult, PgPooledConn,
};

impl EventNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> CustomResult<Event, errors::DatabaseError> {
        generics::generic_insert(conn, self).await
    }
}
