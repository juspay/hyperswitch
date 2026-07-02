use std::future::Future;
use std::pin::Pin;

use bb8::CustomizeConnection;
use common_utils::{
    types::{keymanager, TenantConfig},
    DbConnectionParams,
};
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, PoolError};
use diesel_async::{AsyncConnection, AsyncPgConnection};
use error_stack::ResultExt;

use crate::{
    config::Database,
    errors::{StorageError, StorageResult},
};

pub type PgPool = diesel_async::pooled_connection::bb8::Pool<AsyncPgConnection>;
pub type PgPooledConn = AsyncPgConnection;

#[async_trait::async_trait]
pub trait DatabaseStore: Clone + Send + Sync {
    type Config: Send;
    async fn new(
        config: Self::Config,
        tenant_config: &dyn TenantConfig,
        test_transaction: bool,
        key_manager_state: Option<keymanager::KeyManagerState>,
    ) -> StorageResult<Self>;
    fn get_master_pool(&self) -> &PgPool;
    fn get_replica_pool(&self) -> &PgPool;
    fn get_accounts_master_pool(&self) -> &PgPool;
    fn get_accounts_replica_pool(&self) -> &PgPool;
}

#[derive(Debug, Clone)]
pub struct Store {
    pub master_pool: PgPool,
    pub accounts_pool: PgPool,
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
        Ok(Self {
            master_pool: diesel_make_pg_pool(&config, tenant_config.get_schema(), test_transaction)
                .await?,
            accounts_pool: diesel_make_pg_pool(
                &config,
                tenant_config.get_accounts_schema(),
                test_transaction,
            )
            .await?,
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
}

#[derive(Debug, Clone)]
pub struct ReplicaStore {
    pub master_pool: PgPool,
    pub replica_pool: PgPool,
    pub accounts_master_pool: PgPool,
    pub accounts_replica_pool: PgPool,
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
        let master_pool =
            diesel_make_pg_pool(&master_config, tenant_config.get_schema(), test_transaction)
                .await
                .attach_printable("failed to create master pool")?;
        let accounts_master_pool = diesel_make_pg_pool(
            &master_config,
            tenant_config.get_accounts_schema(),
            test_transaction,
        )
        .await
        .attach_printable("failed to create accounts master pool")?;
        let replica_pool = diesel_make_pg_pool(
            &replica_config,
            tenant_config.get_schema(),
            test_transaction,
        )
        .await
        .attach_printable("failed to create replica pool")?;

        let accounts_replica_pool = diesel_make_pg_pool(
            &replica_config,
            tenant_config.get_accounts_schema(),
            test_transaction,
        )
        .await
        .attach_printable("failed to create accounts pool")?;
        Ok(Self {
            master_pool,
            replica_pool,
            accounts_master_pool,
            accounts_replica_pool,
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
}

pub async fn diesel_make_pg_pool(
    database: &Database,
    schema: &str,
    test_transaction: bool,
) -> StorageResult<PgPool> {
    let database_url = database.get_database_url(schema);
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
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

impl CustomizeConnection<AsyncPgConnection, PoolError> for TestTransaction {
    fn on_acquire<'a>(
        &'a self,
        conn: &'a mut AsyncPgConnection,
    ) -> Pin<Box<dyn Future<Output = Result<(), PoolError>> + Send + 'a>> {
        Box::pin(async move {
            conn.begin_test_transaction()
                .await
                .map_err(PoolError::QueryError)?;
            Ok(())
        })
    }
}
