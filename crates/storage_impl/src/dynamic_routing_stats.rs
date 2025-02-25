use common_utils::errors::CustomResult;
use diesel_models::dynamic_routing_stats as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::dynamic_routing_stats::DynamicRoutingStatsInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> DynamicRoutingStatsInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_dynamic_routing_stat_entry(
        &self,
        dynamic_routing_stat: storage::DynamicRoutingStatsNew,
    ) -> CustomResult<storage::DynamicRoutingStats, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        dynamic_routing_stat
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}
