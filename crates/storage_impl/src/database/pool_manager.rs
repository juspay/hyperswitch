use std::{sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use async_bb8_diesel::ConnectionManager;
use bb8::Pool;
use common_utils::DbConnectionParams;
use diesel::PgConnection;
use error_stack::ResultExt;
use router_env::{logger, tracing::Instrument};
use tokio::sync::Mutex;

use crate::{
    config::Database,
    errors::{StorageError, StorageResult},
};

pub type PgPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Clone)]
pub struct PoolRecoveryConfig {
    pub old_pool_drain_timeout_secs: u64,
}

impl Default for PoolRecoveryConfig {
    fn default() -> Self {
        Self {
            old_pool_drain_timeout_secs: 5,
        }
    }
}

pub struct PgPoolManager {
    pool: Arc<ArcSwap<PgPool>>,
    db_config: Database,
    schema: String,
    test_transaction: bool,
    is_master: bool,
    recovery_config: PoolRecoveryConfig,
    recreation_lock: Arc<Mutex<()>>,
    linked_replica: Arc<std::sync::Mutex<Option<Self>>>,
}

impl std::fmt::Debug for PgPoolManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgPoolManager")
            .field("schema", &self.schema)
            .field("test_transaction", &self.test_transaction)
            .finish()
    }
}

impl Clone for PgPoolManager {
    fn clone(&self) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
            db_config: self.db_config.clone(),
            schema: self.schema.clone(),
            test_transaction: self.test_transaction,
            is_master: self.is_master,
            recovery_config: self.recovery_config.clone(),
            recreation_lock: Arc::clone(&self.recreation_lock),
            linked_replica: Arc::clone(&self.linked_replica),
        }
    }
}

impl PgPoolManager {
    pub async fn new(
        db_config: Database,
        schema: String,
        test_transaction: bool,
        is_master: bool,
        recovery_config: Option<PoolRecoveryConfig>,
    ) -> StorageResult<Self> {
        let config = recovery_config.unwrap_or_default();

        let pool = create_pool(&db_config, &schema, test_transaction).await?;

        Ok(Self {
            pool: Arc::new(ArcSwap::from_pointee(pool)),
            db_config,
            schema,
            test_transaction,
            is_master,
            recovery_config: config,
            recreation_lock: Arc::new(Mutex::new(())),
            linked_replica: Arc::new(std::sync::Mutex::new(None)),
        })
    }

    pub fn get_pool(&self) -> Arc<PgPool> {
        Arc::clone(&self.pool.load())
    }

    pub fn set_linked_replica(&self, replica: Self) {
        if let Ok(mut guard) = self.linked_replica.lock() {
            *guard = Some(replica);
        }
    }

    pub fn check_and_handle_failover_error(&self, error_message: &str) -> bool {
        if is_failover_error(error_message) {
            logger::error!(
                schema = %self.schema,
                error_snippet = &error_message[..error_message.len().min(200)],
                "[FAILOVER] Database failover detected, triggering immediate pool recreation"
            );
            self.trigger_pool_recreation();
            true
        } else {
            false
        }
    }

    async fn recreate_pool(&self) {
        let Ok(_guard) = self.recreation_lock.try_lock() else {
            logger::debug!(
                schema = %self.schema,
                "[POOL_MANAGER] Pool recreation already in progress, skipping"
            );
            return;
        };

        logger::debug!(
            schema = %self.schema,
            db_host = %self.db_config.host,
            "[POOL_MANAGER] Waiting for DNS propagation before creating new pool"
        );

        let dns_delays = [5, 10, 15];

        for (attempt, delay_secs) in dns_delays.iter().enumerate() {
            logger::debug!(
                schema = %self.schema,
                attempt = attempt + 1,
                delay_secs = delay_secs,
                "[POOL_MANAGER] Waiting {}s for DNS propagation (attempt {}/{})",
                delay_secs,
                attempt + 1,
                dns_delays.len()
            );

            tokio::time::sleep(Duration::from_secs(*delay_secs)).await;

            let new_pool =
                match create_pool(&self.db_config, &self.schema, self.test_transaction).await {
                    Ok(pool) => pool,
                    Err(e) => {
                        logger::error!(
                            schema = %self.schema,
                            error = ?e,
                            attempt = attempt + 1,
                            "[POOL_MANAGER] Failed to create new pool, will retry"
                        );
                        continue;
                    }
                };

            if !self.is_master {
                logger::debug!(
                    schema = %self.schema,
                    attempt = attempt + 1,
                    "[POOL_MANAGER] Replica pool created, performing atomic swap"
                );

                let old_pool = self.pool.swap(Arc::new(new_pool));

                let drain_timeout = self.recovery_config.old_pool_drain_timeout_secs;
                tokio::spawn(
                    async move {
                        tokio::time::sleep(Duration::from_secs(drain_timeout)).await;
                        drop(old_pool);
                    }
                    .in_current_span(),
                );

                return;
            }

            match validate_pool_is_writable(&new_pool).await {
                Ok(true) => {
                    logger::debug!(
                        schema = %self.schema,
                        attempt = attempt + 1,
                        "[POOL_MANAGER] New pool validated as writable, performing atomic swap"
                    );

                    let old_pool = self.pool.swap(Arc::new(new_pool));

                    let drain_timeout = self.recovery_config.old_pool_drain_timeout_secs;
                    tokio::spawn(
                        async move {
                            tokio::time::sleep(Duration::from_secs(drain_timeout)).await;
                            drop(old_pool);
                        }
                        .in_current_span(),
                    );

                    if let Ok(guard) = self.linked_replica.lock() {
                        if let Some(replica) = guard.as_ref() {
                            replica.trigger_pool_recreation();
                        }
                    }

                    return;
                }
                Ok(false) => {
                    logger::warn!(
                        schema = %self.schema,
                        attempt = attempt + 1,
                        "[POOL_MANAGER] New pool still connecting to read-only replica, will retry"
                    );
                    continue;
                }
                Err(e) => {
                    logger::warn!(
                        schema = %self.schema,
                        error = %e,
                        attempt = attempt + 1,
                        "[POOL_MANAGER] Failed to validate pool, will retry"
                    );
                    continue;
                }
            }
        }

        logger::error!(
            schema = %self.schema,
            "[POOL_MANAGER] Pool recreation failed after all retries, will retry on next failover error"
        );
    }

    pub fn trigger_pool_recreation(&self) {
        let manager = self.clone();

        tokio::spawn(
            async move {
                manager.recreate_pool().await;
            }
            .in_current_span(),
        );
    }
}

#[allow(unused_qualifications)]
async fn validate_pool_is_writable(pool: &PgPool) -> Result<bool, String> {
    use async_bb8_diesel::AsyncRunQueryDsl;
    use diesel::{prelude::*, sql_query, sql_types::Bool};

    #[derive(QueryableByName, Debug)]
    struct ReadOnlyCheck {
        #[diesel(sql_type = Bool)]
        is_read_only: bool,
    }

    let conn = pool
        .get()
        .await
        .map_err(|e| format!("Failed to get connection: {:?}", e))?;

    let result: Result<ReadOnlyCheck, _> =
        sql_query("SELECT current_setting('transaction_read_only')::boolean AS is_read_only")
            .get_result_async(&*conn)
            .await;

    match result {
        Ok(check) => Ok(!check.is_read_only),
        Err(e) => Err(format!("Query error: {:?}", e)),
    }
}

async fn create_pool(
    database: &Database,
    schema: &str,
    test_transaction: bool,
) -> StorageResult<PgPool> {
    use async_bb8_diesel::AsyncConnection;
    use bb8::CustomizeConnection;

    let database_url = database.get_database_url(schema);
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    let mut pool_builder = Pool::builder()
        .max_size(database.pool_size)
        .min_idle(database.min_idle)
        .queue_strategy(database.queue_strategy.into())
        .connection_timeout(Duration::from_secs(database.connection_timeout))
        .max_lifetime(database.max_lifetime.map(Duration::from_secs));

    if test_transaction {
        #[derive(Debug)]
        struct TestTransaction;

        #[async_trait::async_trait]
        impl
            CustomizeConnection<
                async_bb8_diesel::Connection<PgConnection>,
                async_bb8_diesel::ConnectionError,
            > for TestTransaction
        {
            #[allow(clippy::unwrap_used)]
            async fn on_acquire(
                &self,
                conn: &mut async_bb8_diesel::Connection<PgConnection>,
            ) -> Result<(), async_bb8_diesel::ConnectionError> {
                use diesel::Connection;

                conn.run(|conn| {
                    conn.begin_test_transaction().unwrap();
                    Ok(())
                })
                .await
            }
        }

        pool_builder = pool_builder.connection_customizer(Box::new(TestTransaction));
    }

    pool_builder
        .build(manager)
        .await
        .change_context(StorageError::InitializationError)
        .attach_printable("Failed to create PostgreSQL connection pool")
}

pub fn is_failover_error(error_message: &str) -> bool {
    let error_lower = error_message.to_lowercase();

    error_lower.contains("read-only")
        || error_lower.contains("readonly")
        || (error_lower.contains("cannot execute")
            && (error_lower.contains("insert")
                || error_lower.contains("update")
                || error_lower.contains("delete")
                || error_lower.contains("write")))
        || error_lower.contains("server closed")
        || error_lower.contains("terminating connection")
        || error_lower.contains("connection reset")
        || error_lower.contains("broken pipe")
        || error_lower.contains("connection refused")
        || error_lower.contains("hot standby")
        || error_lower.contains("recovery mode")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_failover_error_read_only() {
        assert!(is_failover_error(
            "cannot execute INSERT in a read-only transaction"
        ));
        assert!(is_failover_error(
            "ERROR: cannot execute UPDATE in a read-only transaction"
        ));
        assert!(is_failover_error("readonly mode"));
    }

    #[test]
    fn test_is_failover_error_connection_terminated() {
        assert!(is_failover_error(
            "server closed the connection unexpectedly"
        ));
        assert!(is_failover_error(
            "terminating connection due to administrator command"
        ));
        assert!(is_failover_error("connection reset by peer"));
        assert!(is_failover_error("broken pipe"));
    }

    #[test]
    fn test_is_failover_error_standby() {
        assert!(is_failover_error(
            "cannot execute in read-only hot standby mode"
        ));
        assert!(is_failover_error("database is in recovery mode"));
    }

    #[test]
    fn test_is_failover_error_non_failover() {
        assert!(!is_failover_error("unique violation"));
        assert!(!is_failover_error("foreign key constraint"));
        assert!(!is_failover_error("not found"));
        assert!(!is_failover_error("syntax error"));
    }
}
