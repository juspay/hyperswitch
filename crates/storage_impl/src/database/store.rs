use async_bb8_diesel::{AsyncConnection, ConnectionError};
use bb8::CustomizeConnection;
use common_utils::{
    types::{keymanager, TenantConfig},
    DbConnectionParams,
};
use diesel_models::DejaPgConnection;
use error_stack::ResultExt;

use crate::{
    config::Database,
    errors::{StorageError, StorageResult},
};

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<DejaPgConnection>>;
pub type PgPooledConn = async_bb8_diesel::Connection<DejaPgConnection>;

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
    ) -> StorageResult<Self> {
        let (master_config, accounts_config) = config;
        Ok(Self {
            master_pool: diesel_make_pg_pool(
                &master_config,
                tenant_config.get_schema(),
                test_transaction,
            )
            .await?,
            accounts_pool: diesel_make_pg_pool(
                &accounts_config,
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
    /// (master config, replica config, accounts master config, accounts replica config)
    type Config = (Database, Database, Database, Database);
    async fn new(
        config: (Database, Database, Database, Database),
        tenant_config: &dyn TenantConfig,
        test_transaction: bool,
        _key_manager_state: Option<keymanager::KeyManagerState>,
    ) -> StorageResult<Self> {
        let (master_config, replica_config, accounts_master_config, accounts_replica_config) =
            config;
        let master_pool =
            diesel_make_pg_pool(&master_config, tenant_config.get_schema(), test_transaction)
                .await
                .attach_printable("failed to create master pool")?;
        let accounts_master_pool = diesel_make_pg_pool(
            &accounts_master_config,
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
            &accounts_replica_config,
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

    let pool = pool
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
        if let Ok(connection) = pool.get().await {
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

    Ok(pool)
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
