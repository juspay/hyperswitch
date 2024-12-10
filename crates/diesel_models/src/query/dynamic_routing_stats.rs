use super::generics;
use crate::{
    dynamic_routing_stats::{DynamicRoutingStats, DynamicRoutingStatsNew},
    PgPooledConn, StorageResult,
};

impl DynamicRoutingStatsNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<DynamicRoutingStats> {
        generics::generic_insert(conn, self).await
    }
}
