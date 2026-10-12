use std::sync::Arc;

use async_bb8_diesel::{AsyncConnection, ConnectionError};
use bb8::CustomizeConnection;
use common_utils::{
    external_service::{ExternalServiceEventEmitter, NoOpEventEmitter},
    request_context::RequestContext,
    types::{keymanager, TenantConfig},
    DbConnectionParams,
};
use diesel_models::{connection_lease, DejaPgConnection, RawPgConnection};
pub use diesel_models::{connection_lease::LeaseTarget, DatabaseConnectionWithContext};
use error_stack::ResultExt;

use crate::{
    config::Database,
    errors::{StorageError, StorageResult},
};

pub type RawPgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<DejaPgConnection>>;

/// A connection leased from a [`PgPool`], before a request context is attached to it.
pub type RawPgPooledConn<'pool> =
    bb8::PooledConnection<'pool, async_bb8_diesel::ConnectionManager<DejaPgConnection>>;

/// What leasing a connection from a [`PgPool`] can fail with.
pub type LeaseError = bb8::RunError<ConnectionError>;

#[derive(Debug, Clone)]
pub struct PgPool {
    /// Private so that every lease goes through [`PgPool::lease`], which is what records
    /// `DATABASE_CONNECTION_LEASE_TIME_MS`. A caller holding the raw pool could lease
    /// without being measured.
    pg_pool: RawPgPool,
    pub event_emitter: Arc<dyn ExternalServiceEventEmitter>,
}

impl PgPool {
    pub fn new(pg_pool: RawPgPool, event_emitter: Arc<dyn ExternalServiceEventEmitter>) -> Self {
        Self {
            pg_pool,
            event_emitter,
        }
    }

    pub fn new_without_event_emitter(pool: RawPgPool) -> Self {
        Self::new(pool, Arc::new(NoOpEventEmitter))
    }

    /// Leases a connection, returning it alongside how long the lease took.
    ///
    /// The only way to get a connection out of a `PgPool`, so that no lease escapes
    /// measurement. The duration goes on to [`DatabaseConnectionWithContext`], which hands
    /// it to the first query run on the connection — that is where the table name becomes
    /// known. See [`diesel_models::connection_lease`] for how a lease ends up attributed.
    ///
    /// A failed lease is recorded here rather than passed on, since there is no connection
    /// to carry it. `TimedOut` (the pool had no slot in time) is kept distinct from `Error`
    /// (no connection could be established): the former is the saturation signal.
    pub async fn lease(
        &self,
        target: LeaseTarget,
    ) -> Result<(RawPgPooledConn<'_>, std::time::Duration), LeaseError> {
        let start = std::time::Instant::now();
        let lease = self.pg_pool.get().await;
        let elapsed = start.elapsed();

        match lease {
            Ok(connection) => Ok((connection, elapsed)),
            Err(error) => {
                let outcome = match &error {
                    bb8::RunError::TimedOut => connection_lease::Outcome::TimedOut,
                    bb8::RunError::User(_) => connection_lease::Outcome::Error,
                };
                connection_lease::record(target, outcome, "none", elapsed);
                Err(error)
            }
        }
    }
}

/// Shared database-pool ownership.
///
/// `RequestContext` is intentionally not a supertrait here — request-scoped wrappers such as
/// `RouterStore` and `KVRouterStore` implement it separately, and `Store`/`ReplicaStore` below
/// never need it. The `pg_*_connection_*` leasing methods instead add `where Self:
/// RequestContext` individually, so they combine the two only where both are actually
/// available.
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

    async fn pg_connection_read<'a>(
        &'a self,
    ) -> error_stack::Result<DatabaseConnectionWithContext<'a>, StorageError>
    where
        Self: RequestContext,
    {
        // If only OLAP is enabled get replica pool.
        #[cfg(all(feature = "olap", not(feature = "oltp")))]
        let (pool, target) = (self.get_replica_pool(), LeaseTarget::PRIMARY_REPLICA_READ);

        // If either one of these are true we need to get master pool.
        //  1. Only OLTP is enabled.
        //  2. Both OLAP and OLTP is enabled.
        //  3. Both OLAP and OLTP is disabled.
        #[cfg(any(
            all(not(feature = "olap"), feature = "oltp"),
            all(feature = "olap", feature = "oltp"),
            all(not(feature = "olap"), not(feature = "oltp"))
        ))]
        let (pool, target) = (self.get_master_pool(), LeaseTarget::PRIMARY_MASTER_READ);

        #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
        let (mut connection, lease_time) = pool
            .lease(target)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;

        #[cfg(feature = "deja")]
        crate::utils::deja_route_replay_schema(&mut connection, self).await;

        Ok(DatabaseConnectionWithContext::new(
            connection,
            self.request_id().map(str::to_owned),
            pool.event_emitter.clone(),
            target,
            lease_time,
        ))
    }

    /// Always leases from the replica pool, irrespective of the `olap`/`oltp` features.
    ///
    /// `pg_connection_read` falls back to the master pool whenever `oltp` is enabled
    /// (including the default build, where both `olap` and `oltp` are on), so analytics
    /// style queries that must never touch master should use this instead. For stores
    /// without a configured replica (`Store`), `get_replica_pool` returns the master
    /// pool, so this stays correct in single-pool deployments.
    async fn pg_connection_read_replica<'a>(
        &'a self,
    ) -> error_stack::Result<DatabaseConnectionWithContext<'a>, StorageError>
    where
        Self: RequestContext,
    {
        let pool = self.get_replica_pool();
        let target = LeaseTarget::PRIMARY_REPLICA_READ;

        #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
        let (mut connection, lease_time) = pool
            .lease(target)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;

        #[cfg(feature = "deja")]
        crate::utils::deja_route_replay_schema(&mut connection, self).await;

        Ok(DatabaseConnectionWithContext::new(
            connection,
            self.request_id().map(str::to_owned),
            pool.event_emitter.clone(),
            target,
            lease_time,
        ))
    }

    async fn pg_connection_write<'a>(
        &'a self,
    ) -> error_stack::Result<DatabaseConnectionWithContext<'a>, StorageError>
    where
        Self: RequestContext,
    {
        // Since all writes should happen to master DB only choose master DB.
        let pool = self.get_master_pool();
        let target = LeaseTarget::PRIMARY_MASTER_WRITE;

        #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
        let (mut connection, lease_time) = pool
            .lease(target)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;

        #[cfg(feature = "deja")]
        crate::utils::deja_route_replay_schema(&mut connection, self).await;

        Ok(DatabaseConnectionWithContext::new(
            connection,
            self.request_id().map(str::to_owned),
            pool.event_emitter.clone(),
            target,
            lease_time,
        ))
    }

    async fn pg_accounts_connection_read<'a>(
        &'a self,
    ) -> error_stack::Result<DatabaseConnectionWithContext<'a>, StorageError>
    where
        Self: RequestContext,
    {
        // If only OLAP is enabled get replica pool.
        #[cfg(all(feature = "olap", not(feature = "oltp")))]
        let (pool, target) = (
            self.get_accounts_replica_pool(),
            LeaseTarget::ACCOUNTS_REPLICA_READ,
        );

        // If either one of these are true we need to get master pool.
        //  1. Only OLTP is enabled.
        //  2. Both OLAP and OLTP is enabled.
        //  3. Both OLAP and OLTP is disabled.
        #[cfg(any(
            all(not(feature = "olap"), feature = "oltp"),
            all(feature = "olap", feature = "oltp"),
            all(not(feature = "olap"), not(feature = "oltp"))
        ))]
        let (pool, target) = (
            self.get_accounts_master_pool(),
            LeaseTarget::ACCOUNTS_MASTER_READ,
        );

        #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
        let (mut connection, lease_time) = pool
            .lease(target)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;

        #[cfg(feature = "deja")]
        crate::utils::deja_route_replay_schema(&mut connection, self).await;

        Ok(DatabaseConnectionWithContext::new(
            connection,
            self.request_id().map(str::to_owned),
            pool.event_emitter.clone(),
            target,
            lease_time,
        ))
    }

    async fn pg_accounts_connection_write<'a>(
        &'a self,
    ) -> error_stack::Result<DatabaseConnectionWithContext<'a>, StorageError>
    where
        Self: RequestContext,
    {
        // Since all writes should happen to master DB only choose master DB.
        let pool = self.get_accounts_master_pool();
        let target = LeaseTarget::ACCOUNTS_MASTER_WRITE;

        #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
        let (mut connection, lease_time) = pool
            .lease(target)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;

        #[cfg(feature = "deja")]
        crate::utils::deja_route_replay_schema(&mut connection, self).await;

        Ok(DatabaseConnectionWithContext::new(
            connection,
            self.request_id().map(str::to_owned),
            pool.event_emitter.clone(),
            target,
            lease_time,
        ))
    }

    /// Request correlation used by deja replay to route database connections to
    /// the active replay schema. Stores without request identity return `None`.
    #[cfg(feature = "deja")]
    fn get_request_id(&self) -> Option<String> {
        None
    }
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
    let manager = async_bb8_diesel::ConnectionManager::<DejaPgConnection>::new(database_url);
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

    // Register row identity (primary-key columns) with deja from this
    // database's own catalog. Idempotent; on failure identity stays
    // unregistered, making recorded row keys absent rather than wrong.
    #[cfg(feature = "deja")]
    if !deja::runtime_mode_is_disabled() {
        use async_bb8_diesel::AsyncConnection;
        use diesel::RunQueryDsl;
        if let Ok(connection) = raw_pool.get().await {
            let rows = connection
                .run(|conn| {
                    diesel::sql_query(deja::TABLE_IDENTITY_SQL)
                        .load::<deja::db::TableIdentityRow>(conn)
                        .map(|rows| {
                            rows.into_iter()
                                .map(|row| (row.table_name, row.column_name))
                                .collect::<Vec<(String, String)>>()
                        })
                })
                .await;
            match rows {
                Ok(rows) => deja::db::register_table_identity_rows(rows),
                Err(error) => {
                    router_env::logger::warn!(
                        ?error,
                        "deja: could not read row identity from the schema; recorded row keys will fall back to query fingerprints"
                    );
                }
            }
        }
    }

    Ok(PgPool::new(raw_pool, event_emitter))
}

#[derive(Debug)]
struct TestTransaction;

#[async_trait::async_trait]
impl CustomizeConnection<RawPgConnection, ConnectionError> for TestTransaction {
    #[allow(clippy::unwrap_used)]
    async fn on_acquire(&self, conn: &mut RawPgConnection) -> Result<(), ConnectionError> {
        use diesel::Connection;

        conn.run(|conn| {
            conn.begin_test_transaction().unwrap();
            Ok(())
        })
        .await
    }
}
