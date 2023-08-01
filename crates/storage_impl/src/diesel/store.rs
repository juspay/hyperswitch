use async_bb8_diesel::{AsyncConnection, ConnectionError};
use bb8::CustomizeConnection;
use diesel::PgConnection;
use masking::PeekInterface;

use crate::config::Database;

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;
pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

#[derive(Clone)]
pub struct Store {
    pub master_pool: PgPool,
}

impl Store {
    pub async fn new(config: &Database, test_transaction: bool) -> Self {
        Self {
            master_pool: diesel_make_pg_pool(config, test_transaction).await,
        }
    }
}

#[derive(Clone)]
pub struct ReplicaStore {
    pub master_pool: PgPool,
    pub replica_pool: PgPool,
}

impl ReplicaStore {
    pub async fn new(
        master_config: &Database,
        replica_config: &Database,
        test_transaction: bool,
    ) -> Self {
        let master_pool = diesel_make_pg_pool(master_config, test_transaction).await;
        let replica_pool = diesel_make_pg_pool(replica_config, test_transaction).await;
        Self {
            master_pool,
            replica_pool,
        }
    }
}

#[allow(clippy::expect_used)]
pub async fn diesel_make_pg_pool(database: &Database, test_transaction: bool) -> PgPool {
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
        .connection_timeout(std::time::Duration::from_secs(database.connection_timeout));

    if test_transaction {
        pool = pool.connection_customizer(Box::new(TestTransaction));
    }

    pool.build(manager)
        .await
        .expect("Failed to create PostgreSQL connection pool")
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
