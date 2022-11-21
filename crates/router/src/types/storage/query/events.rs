use router_env::tracing::{self, instrument};

use super::generics::{self, ExecuteQuery};
use crate::{
    connection::PgPooledConn,
    core::errors::{self, CustomResult},
    types::storage::{Event, EventNew},
};

impl EventNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> CustomResult<Event, errors::StorageError> {
        generics::generic_insert::<_, _, Event, _>(conn, self, ExecuteQuery::new()).await
    }
}
