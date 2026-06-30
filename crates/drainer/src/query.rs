use std::sync::Arc;

use common_utils::errors::CustomResult;
use diesel_models::errors::DatabaseError;

use crate::{kv, logger, metrics, pg_connection, services::Store};

#[async_trait::async_trait]
pub trait ExecuteQuery {
    async fn execute_query(
        self,
        store: &Arc<Store>,
        pushed_at: i64,
    ) -> CustomResult<(), DatabaseError>;
}

#[async_trait::async_trait]
impl ExecuteQuery for kv::SerializableQuery {
    async fn execute_query(
        self,
        store: &Arc<Store>,
        pushed_at: i64,
    ) -> CustomResult<(), DatabaseError> {
        let mut conn = pg_connection(&store.master_pool).await;
        let operation = self.operation().to_string();
        let entity_type = self.entity_type();

        let metric_attributes = router_env::metric_attributes!(
            ("operation", operation.clone()),
            ("entity_type", entity_type.clone())
        );

        let (result, execution_time) =
            common_utils::date_time::time_it(|| self.execute(&mut conn)).await;

        push_drainer_delay(pushed_at, &operation, &entity_type, metric_attributes);
        metrics::QUERY_EXECUTION_TIME.record(execution_time, metric_attributes);

        match result {
            Ok(rows_affected) => {
                logger::info!(operation, entity_type, ?rows_affected);
                metrics::SUCCESSFUL_QUERY_EXECUTION.add(1, metric_attributes);
                Ok(())
            }
            Err(error) => {
                logger::error!(operation, entity_type, ?error);
                metrics::ERRORS_WHILE_QUERY_EXECUTION.add(1, metric_attributes);
                Err(error)
            }
        }
    }
}

#[inline(always)]
fn push_drainer_delay(
    pushed_at: i64,
    operation: &str,
    entity_type: &str,
    metric_attributes: &[router_env::opentelemetry::KeyValue],
) {
    let drained_at = common_utils::date_time::now_unix_timestamp();
    let delay = drained_at - pushed_at;

    logger::debug!(operation, entity_type, delay = format!("{delay} secs"));

    match u64::try_from(delay) {
        Ok(delay) => metrics::DRAINER_DELAY_SECONDS.record(delay, metric_attributes),
        Err(error) => logger::error!(
            pushed_at,
            drained_at,
            delay,
            ?error,
            "Invalid drainer delay"
        ),
    }
}
