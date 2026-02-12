use std::sync::Arc;

use async_bb8_diesel::{AsyncConnection, ConnectionError};
use bb8::CustomizeConnection;
use common_utils::{
    types::{keymanager, TenantConfig},
    DbConnectionParams,
};
use diesel::PgConnection;
use error_stack::ResultExt;

use super::pool_manager::{PgPoolManager, PoolRecoveryConfig};
use crate::{
    config::Database,
    errors::{StorageError, StorageResult},
};

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;
pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

/// Trait for database store implementations.
///
/// This trait provides access to PostgreSQL connection pools and pool managers.
/// The pool managers provide automatic retry and failover recovery capabilities.
#[async_trait::async_trait]
pub trait DatabaseStore: Clone + Send + Sync {
    type Config: Send;

    /// Creates a new database store with the given configuration.
    async fn new(
        config: Self::Config,
        tenant_config: &dyn TenantConfig,
        test_transaction: bool,
        key_manager_state: Option<keymanager::KeyManagerState>,
    ) -> StorageResult<Self>;

    /// Returns a reference to the master pool.
    ///
    /// Note: This pool reference may become stale during failover recovery.
    /// For production use with failover support, prefer using `get_master_pool_manager()`
    /// and calling `get_pool()` on it for each connection request.
    fn get_master_pool(&self) -> &PgPool;

    /// Returns a reference to the replica pool.
    ///
    /// Note: This pool reference may become stale during failover recovery.
    /// For production use with failover support, prefer using `get_replica_pool_manager()`
    /// and calling `get_pool()` on it for each connection request.
    fn get_replica_pool(&self) -> &PgPool;

    /// Returns a reference to the accounts master pool.
    fn get_accounts_master_pool(&self) -> &PgPool;

    /// Returns a reference to the accounts replica pool.
    fn get_accounts_replica_pool(&self) -> &PgPool;

    /// Gets the pool manager for the master database.
    ///
    /// The pool manager provides:
    /// - Lock-free atomic pool access via `get_pool()`
    /// - Failure tracking and automatic pool recreation on failover
    ///
    /// Use this for code paths that need failover resilience.
    fn get_master_pool_manager(&self) -> &PgPoolManager;

    /// Gets the pool manager for the replica database.
    fn get_replica_pool_manager(&self) -> &PgPoolManager;

    /// Gets the pool manager for the accounts master database.
    fn get_accounts_master_pool_manager(&self) -> &PgPoolManager;

    /// Gets the pool manager for the accounts replica database.
    fn get_accounts_replica_pool_manager(&self) -> &PgPoolManager;

    /// Handles database query errors and takes appropriate recovery actions.
    ///
    /// This should be called when a database query fails. Currently handles:
    /// - Failover detection (e.g., "read-only transaction") - triggers pool recreation
    ///
    /// Returns `true` if any recovery action was triggered.
    fn handle_query_error(&self, error_message: &str) -> bool {
        let master_triggered = self
            .get_master_pool_manager()
            .check_and_handle_failover_error(error_message);

        let accounts_triggered = self
            .get_accounts_master_pool_manager()
            .check_and_handle_failover_error(error_message);

        master_triggered || accounts_triggered
    }
}

/// Store with a single database (master only, used for both reads and writes).
#[derive(Clone)]
pub struct Store {
    master_pool: Arc<PgPool>,
    master_pool_manager: PgPoolManager,
    accounts_pool: Arc<PgPool>,
    accounts_pool_manager: PgPoolManager,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store")
            .field("master_pool_manager", &self.master_pool_manager)
            .field("accounts_pool_manager", &self.accounts_pool_manager)
            .finish()
    }
}

#[async_trait::async_trait]
impl DatabaseStore for Store {
    type Config = Database;

    async fn new(
        config: Database,
        tenant_config: &dyn TenantConfig,
        test_transaction: bool,
        _key_manager_state: Option<keymanager::KeyManagerState>,
    ) -> StorageResult<Self> {
        let recovery_config = PoolRecoveryConfig::default();

        let master_pool_manager = PgPoolManager::new(
            config.clone(),
            tenant_config.get_schema().to_string(),
            test_transaction,
            Some(recovery_config.clone()),
        )
        .await
        .attach_printable("failed to create master pool manager")?;

        let accounts_pool_manager = PgPoolManager::new(
            config,
            tenant_config.get_accounts_schema().to_string(),
            test_transaction,
            Some(recovery_config),
        )
        .await
        .attach_printable("failed to create accounts pool manager")?;

        let master_pool = master_pool_manager.get_pool();
        let accounts_pool = accounts_pool_manager.get_pool();

        Ok(Self {
            master_pool,
            master_pool_manager,
            accounts_pool,
            accounts_pool_manager,
        })
    }

    fn get_master_pool(&self) -> &PgPool {
        &self.master_pool
    }

    fn get_replica_pool(&self) -> &PgPool {
        &self.master_pool
    }

    fn get_accounts_master_pool(&self) -> &PgPool {
        &self.accounts_pool
    }

    fn get_accounts_replica_pool(&self) -> &PgPool {
        &self.accounts_pool
    }

    fn get_master_pool_manager(&self) -> &PgPoolManager {
        &self.master_pool_manager
    }

    fn get_replica_pool_manager(&self) -> &PgPoolManager {
        &self.master_pool_manager
    }

    fn get_accounts_master_pool_manager(&self) -> &PgPoolManager {
        &self.accounts_pool_manager
    }

    fn get_accounts_replica_pool_manager(&self) -> &PgPoolManager {
        &self.accounts_pool_manager
    }
}

/// Store with separate master and replica databases.
#[derive(Clone)]
pub struct ReplicaStore {
    master_pool: Arc<PgPool>,
    master_pool_manager: PgPoolManager,
    replica_pool: Arc<PgPool>,
    replica_pool_manager: PgPoolManager,
    accounts_master_pool: Arc<PgPool>,
    accounts_master_pool_manager: PgPoolManager,
    accounts_replica_pool: Arc<PgPool>,
    accounts_replica_pool_manager: PgPoolManager,
}

impl std::fmt::Debug for ReplicaStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplicaStore")
            .field("master_pool_manager", &self.master_pool_manager)
            .field("replica_pool_manager", &self.replica_pool_manager)
            .field(
                "accounts_master_pool_manager",
                &self.accounts_master_pool_manager,
            )
            .field(
                "accounts_replica_pool_manager",
                &self.accounts_replica_pool_manager,
            )
            .finish()
    }
}

#[async_trait::async_trait]
impl DatabaseStore for ReplicaStore {
    type Config = (Database, Database);

    async fn new(
        config: (Database, Database),
        tenant_config: &dyn TenantConfig,
        test_transaction: bool,
        _key_manager_state: Option<keymanager::KeyManagerState>,
    ) -> StorageResult<Self> {
        let (master_config, replica_config) = config;
        let recovery_config = PoolRecoveryConfig::default();

        let master_pool_manager = PgPoolManager::new(
            master_config.clone(),
            tenant_config.get_schema().to_string(),
            test_transaction,
            Some(recovery_config.clone()),
        )
        .await
        .attach_printable("failed to create master pool manager")?;

        let accounts_master_pool_manager = PgPoolManager::new(
            master_config,
            tenant_config.get_accounts_schema().to_string(),
            test_transaction,
            Some(recovery_config.clone()),
        )
        .await
        .attach_printable("failed to create accounts master pool manager")?;

        let replica_pool_manager = PgPoolManager::new(
            replica_config.clone(),
            tenant_config.get_schema().to_string(),
            test_transaction,
            Some(recovery_config.clone()),
        )
        .await
        .attach_printable("failed to create replica pool manager")?;

        let accounts_replica_pool_manager = PgPoolManager::new(
            replica_config,
            tenant_config.get_accounts_schema().to_string(),
            test_transaction,
            Some(recovery_config),
        )
        .await
        .attach_printable("failed to create accounts replica pool manager")?;

        let master_pool = master_pool_manager.get_pool();
        let replica_pool = replica_pool_manager.get_pool();
        let accounts_master_pool = accounts_master_pool_manager.get_pool();
        let accounts_replica_pool = accounts_replica_pool_manager.get_pool();

        Ok(Self {
            master_pool,
            master_pool_manager,
            replica_pool,
            replica_pool_manager,
            accounts_master_pool,
            accounts_master_pool_manager,
            accounts_replica_pool,
            accounts_replica_pool_manager,
        })
    }

    fn get_master_pool(&self) -> &PgPool {
        &self.master_pool
    }

    fn get_replica_pool(&self) -> &PgPool {
        &self.replica_pool
    }

    fn get_accounts_master_pool(&self) -> &PgPool {
        &self.accounts_master_pool
    }

    fn get_accounts_replica_pool(&self) -> &PgPool {
        &self.accounts_replica_pool
    }

    fn get_master_pool_manager(&self) -> &PgPoolManager {
        &self.master_pool_manager
    }

    fn get_replica_pool_manager(&self) -> &PgPoolManager {
        &self.replica_pool_manager
    }

    fn get_accounts_master_pool_manager(&self) -> &PgPoolManager {
        &self.accounts_master_pool_manager
    }

    fn get_accounts_replica_pool_manager(&self) -> &PgPoolManager {
        &self.accounts_replica_pool_manager
    }
}

/// Creates a PostgreSQL connection pool directly (without PgPoolManager wrapper).
/// This is used internally by PgPoolManager.
pub async fn diesel_make_pg_pool(
    database: &Database,
    schema: &str,
    test_transaction: bool,
) -> StorageResult<PgPool> {
    let database_url = database.get_database_url(schema);
    let manager = async_bb8_diesel::ConnectionManager::<PgConnection>::new(database_url);
    let mut pool = bb8::Pool::builder()
        .max_size(database.pool_size)
        .min_idle(database.min_idle)
        .queue_strategy(database.queue_strategy.into())
        .connection_timeout(std::time::Duration::from_secs(database.connection_timeout))
        .max_lifetime(database.max_lifetime.map(std::time::Duration::from_secs));

    if test_transaction {
        pool = pool.connection_customizer(Box::new(TestTransaction));
    }

    pool.build(manager)
        .await
        .change_context(StorageError::InitializationError)
        .attach_printable("Failed to create PostgreSQL connection pool")
}

#[derive(Debug)]
struct TestTransaction;

#[async_trait::async_trait]
impl CustomizeConnection<PgPooledConn, ConnectionError> for TestTransaction {
    #[allow(clippy::unwrap_used)]
    async fn on_acquire(&self, conn: &mut PgPooledConn) -> Result<(), ConnectionError> {
        use diesel::Connection;

        conn.run(|conn| {
            conn.begin_test_transaction().unwrap();
            Ok(())
        })
        .await
    }
}
