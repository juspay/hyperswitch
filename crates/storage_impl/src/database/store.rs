use std::sync::Arc;

use async_bb8_diesel::{AsyncConnection, ConnectionError};
use bb8::{CustomizeConnection, PooledConnection};
use common_utils::{
    external_service::{ExternalServiceEventEmitter, NoOpEventEmitter},
    request_context::RequestContext,
    types::{keymanager, TenantConfig},
    DbConnectionParams,
};
use diesel::PgConnection;
use diesel_models::{DatabaseConnection, DatabaseEventContext, RawPgConnection};
use error_stack::ResultExt;

use crate::{
    config::Database,
    errors::{StorageError, StorageResult},
};

pub type PgConnectionManager = async_bb8_diesel::ConnectionManager<PgConnection>;
pub type RawPgPool = bb8::Pool<PgConnectionManager>;
pub type RawPgPooledConn = async_bb8_diesel::Connection<PgConnection>;

#[derive(Debug, Clone)]
pub struct PgPool {
    inner: RawPgPool,
    event_emitter: Arc<dyn ExternalServiceEventEmitter>,
}

impl PgPool {
    pub fn new(inner: RawPgPool, event_emitter: Arc<dyn ExternalServiceEventEmitter>) -> Self {
        Self {
            inner,
            event_emitter,
        }
    }

    pub fn new_without_event_emitter(inner: RawPgPool) -> Self {
        Self::new(inner, Arc::new(NoOpEventEmitter))
    }

    pub async fn get_without_context(
        &self,
    ) -> Result<DatabaseConnectionWithContext, bb8::RunError<ConnectionError>> {
        self.get_with_request_id(None).await
    }

    async fn get_with_request_id(
        &self,
        request_id: Option<String>,
    ) -> Result<DatabaseConnectionWithContext, bb8::RunError<ConnectionError>> {
        let connection = self.inner.get_owned().await?;
        Ok(DatabaseConnectionWithContext {
            connection,
            event_context: DatabaseEventContext::new(request_id, Arc::clone(&self.event_emitter)),
        })
    }
}

pub struct DatabaseConnectionWithContext {
    connection: PooledConnection<'static, PgConnectionManager>,
    event_context: DatabaseEventContext,
}

impl DatabaseConnection for DatabaseConnectionWithContext {
    fn raw_connection(&self) -> &RawPgConnection {
        &self.connection
    }

    fn event_context(&self) -> &DatabaseEventContext {
        &self.event_context
    }
}

pub struct DatabaseTransactionConnectionWithContext {
    connection: RawPgConnection,
    event_context: DatabaseEventContext,
}

impl DatabaseTransactionConnectionWithContext {
    pub fn new(connection: RawPgConnection, event_context: DatabaseEventContext) -> Self {
        Self {
            connection,
            event_context,
        }
    }
}

impl DatabaseConnection for DatabaseTransactionConnectionWithContext {
    fn raw_connection(&self) -> &RawPgConnection {
        &self.connection
    }

    fn event_context(&self) -> &DatabaseEventContext {
        &self.event_context
    }
}

/// Shared database-pool ownership.
///
/// Request context is intentionally not part of this trait. Request-scoped
/// wrappers such as `RouterStore` and `KVRouterStore` implement
/// [`RequestContext`] separately and opt into [`DatabaseStoreWithContext`].
#[async_trait::async_trait]
pub trait DatabaseStore: Clone + Send + Sync {
    type Config: Send;
    async fn new(
        config: Self::Config,
        tenant_config: &dyn TenantConfig,
        test_transaction: bool,
        key_manager_state: Option<keymanager::KeyManagerState>,
        event_emitter: Arc<dyn ExternalServiceEventEmitter>,
    ) -> StorageResult<Self>;
    fn get_master_pool(&self) -> &PgPool;
    fn get_replica_pool(&self) -> &PgPool;
    fn get_accounts_master_pool(&self) -> &PgPool;
    fn get_accounts_replica_pool(&self) -> &PgPool;

    /// Request correlation used by deja replay to route database connections to
    /// the active replay schema. Stores without request identity return `None`.
    #[cfg(feature = "deja")]
    fn get_request_id(&self) -> Option<String> {
        None
    }
}

/// Request-scoped database connection checkout.
///
/// This trait is implemented only by stores that carry a request ID, such as
/// `RouterStore` and `KVRouterStore`. The underlying `Store` and
/// `ReplicaStore` types only own pools and do not implement this trait.
#[async_trait::async_trait]
pub trait DatabaseStoreWithContext: DatabaseStore + RequestContext {
    async fn get_read_connection(&self) -> StorageResult<DatabaseConnectionWithContext> {
        checkout_with_context(self.get_replica_pool(), self).await
    }

    async fn get_write_connection(&self) -> StorageResult<DatabaseConnectionWithContext> {
        checkout_with_context(self.get_master_pool(), self).await
    }

    async fn get_accounts_read_connection(&self) -> StorageResult<DatabaseConnectionWithContext> {
        checkout_with_context(self.get_accounts_replica_pool(), self).await
    }

    async fn get_accounts_write_connection(&self) -> StorageResult<DatabaseConnectionWithContext> {
        checkout_with_context(self.get_accounts_master_pool(), self).await
    }
}

async fn checkout_with_context(
    pool: &PgPool,
    context: &dyn RequestContext,
) -> StorageResult<DatabaseConnectionWithContext> {
    pool.get_with_request_id(context.request_id().map(str::to_owned))
        .await
        .change_context(StorageError::DatabaseConnectionError)
}

#[derive(Debug, Clone)]
pub struct Store {
    pub master_pool: PgPool,
    pub accounts_pool: PgPool,
}

#[async_trait::async_trait]
impl DatabaseStore for Store {
    /// (master config, accounts config)
    type Config = (Database, Database);
    async fn new(
        config: (Database, Database),
        tenant_config: &dyn TenantConfig,
        test_transaction: bool,
        _key_manager_state: Option<keymanager::KeyManagerState>,
        event_emitter: Arc<dyn ExternalServiceEventEmitter>,
    ) -> StorageResult<Self> {
        let (master_config, accounts_config) = config;
        Ok(Self {
            master_pool: diesel_make_pg_pool(
                &master_config,
                tenant_config.get_schema(),
                test_transaction,
                Arc::clone(&event_emitter),
            )
            .await?,
            accounts_pool: diesel_make_pg_pool(
                &accounts_config,
                tenant_config.get_accounts_schema(),
                test_transaction,
                event_emitter,
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
    /// (master config, replica config, accounts master config, accounts replica config)
    type Config = (Database, Database, Database, Database);
    async fn new(
        config: (Database, Database, Database, Database),
        tenant_config: &dyn TenantConfig,
        test_transaction: bool,
        _key_manager_state: Option<keymanager::KeyManagerState>,
        event_emitter: Arc<dyn ExternalServiceEventEmitter>,
    ) -> StorageResult<Self> {
        let (master_config, replica_config, accounts_master_config, accounts_replica_config) =
            config;
        let master_pool = diesel_make_pg_pool(
            &master_config,
            tenant_config.get_schema(),
            test_transaction,
            Arc::clone(&event_emitter),
        )
        .await
        .attach_printable("failed to create master pool")?;
        let accounts_master_pool = diesel_make_pg_pool(
            &accounts_master_config,
            tenant_config.get_accounts_schema(),
            test_transaction,
            Arc::clone(&event_emitter),
        )
        .await
        .attach_printable("failed to create accounts master pool")?;
        let replica_pool = diesel_make_pg_pool(
            &replica_config,
            tenant_config.get_schema(),
            test_transaction,
            Arc::clone(&event_emitter),
        )
        .await
        .attach_printable("failed to create replica pool")?;

        let accounts_replica_pool = diesel_make_pg_pool(
            &accounts_replica_config,
            tenant_config.get_accounts_schema(),
            test_transaction,
            event_emitter,
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
    event_emitter: Arc<dyn ExternalServiceEventEmitter>,
) -> StorageResult<PgPool> {
    let database_url = database.get_database_url(schema);
    let manager = async_bb8_diesel::ConnectionManager::<PgConnection>::new(database_url);
    let mut pool = bb8::Pool::builder()
        .max_size(database.max_pool_size)
        .min_idle(Some(database.min_idle_pool_size))
        .queue_strategy(database.queue_strategy.into())
        .connection_timeout(std::time::Duration::from_secs(database.connection_timeout))
        .max_lifetime(std::time::Duration::from_secs(database.max_lifetime))
        .idle_timeout(std::time::Duration::from_secs(database.idle_timeout));

    if test_transaction {
        pool = pool.connection_customizer(Box::new(TestTransaction));
    }

    let raw_pool = pool
        .build(manager)
        .await
        .change_context(StorageError::InitializationError)
        .attach_printable("Failed to create PostgreSQL connection pool")?;

    Ok(PgPool::new(raw_pool, event_emitter))
}

#[derive(Debug)]
struct TestTransaction;

#[async_trait::async_trait]
impl CustomizeConnection<RawPgPooledConn, ConnectionError> for TestTransaction {
    #[allow(clippy::unwrap_used)]
    async fn on_acquire(&self, conn: &mut RawPgPooledConn) -> Result<(), ConnectionError> {
        use diesel::Connection;

        conn.run(|conn| {
            conn.begin_test_transaction().unwrap();
            Ok(())
        })
        .await
    }
}
