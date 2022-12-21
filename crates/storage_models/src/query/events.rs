use router_env::{instrument, tracing};

use super::generics;
use crate::{
    events::{Event, EventNew},
    PgPooledConn, StorageResult,
};

impl EventNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Event> {
        generics::generic_insert(conn, self).await
    }
}
