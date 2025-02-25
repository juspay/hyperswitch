// use error_stack::report;
// use router_env::{instrument, tracing};
// use storage_impl::MockDb;

// use super::Store;
// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     db::kafka_store::KafkaStore,
//     types::storage,
// };

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;
use diesel_models::dynamic_routing_stats as storage;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait DynamicRoutingStatsInterface {
    type Error;
    async fn insert_dynamic_routing_stat_entry(
        &self,
        dynamic_routing_stat_new: storage::DynamicRoutingStatsNew,
    ) -> CustomResult<storage::DynamicRoutingStats, Self::Error>;
}

// #[async_trait::async_trait]
// impl DynamicRoutingStatsInterface for Store {
//     #[instrument(skip_all)]
//     async fn insert_dynamic_routing_stat_entry(
//         &self,
//         dynamic_routing_stat: storage::DynamicRoutingStatsNew,
//     ) -> CustomResult<storage::DynamicRoutingStats, errors::StorageError> {
//         let conn = connection::pg_connection_write(self).await?;
//         dynamic_routing_stat
//             .insert(&conn)
//             .await
//             .map_err(|error| report!(errors::StorageError::from(error)))
//     }
// }

// #[async_trait::async_trait]
// impl DynamicRoutingStatsInterface for MockDb {
//     #[instrument(skip_all)]
//     async fn insert_dynamic_routing_stat_entry(
//         &self,
//         _dynamic_routing_stat: storage::DynamicRoutingStatsNew,
//     ) -> CustomResult<storage::DynamicRoutingStats, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }
// }

// #[async_trait::async_trait]
// impl DynamicRoutingStatsInterface for KafkaStore {
//     #[instrument(skip_all)]
//     async fn insert_dynamic_routing_stat_entry(
//         &self,
//         dynamic_routing_stat: storage::DynamicRoutingStatsNew,
//     ) -> CustomResult<storage::DynamicRoutingStats, errors::StorageError> {
//         self.diesel_store
//             .insert_dynamic_routing_stat_entry(dynamic_routing_stat)
//             .await
//     }
// }
