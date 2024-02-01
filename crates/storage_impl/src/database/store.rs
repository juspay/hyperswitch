use async_bb8_diesel::{AsyncConnection, ConnectionError};
use bb8::CustomizeConnection;
use data_models::errors::{StorageError, StorageResult};
use diesel::PgConnection;
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;

use crate::config::Database;

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;
pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

#[async_trait::async_trait]
pub trait DatabaseStore: Clone + Send + Sync {
    type Config: Send;
    async fn new(config: Self::Config, test_transaction: bool) -> StorageResult<Self>;
    fn get_master_pool(&self) -> &PgPool;
    fn get_replica_pool(&self) -> &PgPool;
}

#[derive(Debug, Clone)]
pub struct Store {
    pub master_pool: PgPool,
}

#[async_trait::async_trait]
impl DatabaseStore for Store {
    type Config = Database;
    /// Creates a new instance of Storage with the provided database configuration and test transaction flag.
    /// 
    /// # Arguments
    /// 
    /// * `config` - A Database struct containing the configuration for the database.
    /// * `test_transaction` - A boolean flag indicating whether to use a test transaction.
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the newly created Storage instance, or an error if the database connection pool creation fails.
    /// 
    async fn new(config: Database, test_transaction: bool) -> StorageResult<Self> {
        Ok(Self {
            master_pool: diesel_make_pg_pool(&config, test_transaction).await?,
        })
    }
    /// Returns a reference to the master PgPool owned by the current instance.
    fn get_master_pool(&self) -> &PgPool {
        &self.master_pool
    }

    /// Returns a reference to the replica pool associated with the current connection pool.
    fn get_replica_pool(&self) -> &PgPool {
        &self.master_pool
    }
}

#[derive(Debug, Clone)]
pub struct ReplicaStore {
    pub master_pool: PgPool,
    pub replica_pool: PgPool,
}

#[async_trait::async_trait]
impl DatabaseStore for ReplicaStore {
    type Config = (Database, Database);
    /// Asynchronously creates a new Storage instance using the provided database configurations for master and replica databases. If test_transaction is true, a test transaction will be used. Returns a StorageResult containing the newly created Storage instance if successful, or an error otherwise.
    async fn new(config: (Database, Database), test_transaction: bool) -> StorageResult<Self> {
        let (master_config, replica_config) = config;
        let master_pool = diesel_make_pg_pool(&master_config, test_transaction)
            .await
            .attach_printable("failed to create master pool")?;
        let replica_pool = diesel_make_pg_pool(&replica_config, test_transaction)
            .await
            .attach_printable("failed to create replica pool")?;
        Ok(Self {
            master_pool,
            replica_pool,
        })
    }

        /// Retrieves a reference to the master connection pool for PostgreSQL database.
    fn get_master_pool(&self) -> &PgPool {
            &self.master_pool
    }

    /// This method returns a reference to the replica pool associated with the current instance.
    fn get_replica_pool(&self) -> &PgPool {
        &self.replica_pool
    }
}

/// Creates a PostgreSQL connection pool using the given database configuration. If test_transaction is true, the pool will be customized to use a test transaction during connection. Returns a StorageResult containing the initialized PgPool.
pub async fn diesel_make_pg_pool(
    database: &Database,
    test_transaction: bool,
) -> StorageResult<PgPool> {
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        database.username,
        database.password.peek(),
        database.host,
        database.port,
        database.dbname
    );
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
        .into_report()
        .change_context(StorageError::InitializationError)
        .attach_printable("Failed to create PostgreSQL connection pool")
}

#[derive(Debug)]
struct TestTransaction;

#[async_trait::async_trait]
impl CustomizeConnection<PgPooledConn, ConnectionError> for TestTransaction {
    #[allow(clippy::unwrap_used)]
        /// Asynchronously starts a test transaction on the acquired PostgreSQL connection.
    async fn on_acquire(&self, conn: &mut PgPooledConn) -> Result<(), ConnectionError> {
        use diesel::Connection;

        conn.run(|conn| {
            conn.begin_test_transaction().unwrap();
            Ok(())
        })
        .await
    }
}
