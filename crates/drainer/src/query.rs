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
impl ExecuteQuery for kv::DBOperation {
    async fn execute_query(
        self,
        store: &Arc<Store>,
        pushed_at: i64,
    ) -> CustomResult<(), DatabaseError> {
        let conn = pg_connection(&store.master_pool).await;
        let operation = self.operation();
        let table = self.table();

        let tags: &[metrics::KeyValue] = &[
            metrics::KeyValue {
                key: "operation".into(),
                value: operation.into(),
            },
            metrics::KeyValue {
                key: "table".into(),
                value: table.into(),
            },
        ];

        let (result, execution_time) =
            common_utils::date_time::time_it(|| self.execute(&conn)).await;

        push_drainer_delay(pushed_at, operation, table, tags);
        metrics::QUERY_EXECUTION_TIME.record(&metrics::CONTEXT, execution_time, tags);

        match result {
            Ok(result) => {
                logger::info!(operation = operation, table = table, ?result);
                metrics::SUCCESSFUL_QUERY_EXECUTION.add(&metrics::CONTEXT, 1, tags);
                Ok(())
            }
            Err(err) => {
                logger::error!(operation = operation, table = table, ?err);
                metrics::ERRORS_WHILE_QUERY_EXECUTION.add(&metrics::CONTEXT, 1, tags);
                Err(err)
            }
        }
    }
}

#[inline(always)]
fn push_drainer_delay(pushed_at: i64, operation: &str, table: &str, tags: &[metrics::KeyValue]) {
    let drained_at = common_utils::date_time::now_unix_timestamp();
    let delay = drained_at - pushed_at;

    logger::debug!(
        operation = operation,
        table = table,
        delay = format!("{delay} secs")
    );

    metrics::DRAINER_DELAY_SECONDS.record(&metrics::CONTEXT, delay, tags);
}
