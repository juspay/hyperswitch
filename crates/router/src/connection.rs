use async_bb8_diesel::{AsyncConnection, ConnectionError};
use bb8::{CustomizeConnection, PooledConnection};
use diesel::PgConnection;

use crate::configs::settings::Database;

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;

pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;
pub type RedisPool = std::sync::Arc<redis_interface::RedisConnectionPool>;

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

pub async fn redis_connection(
    conf: &crate::configs::settings::Settings,
) -> redis_interface::RedisConnectionPool {
    redis_interface::RedisConnectionPool::new(&conf.redis).await
}

#[allow(clippy::expect_used)]
pub async fn diesel_make_pg_pool(database: &Database, test_transaction: bool) -> PgPool {
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        database.username, database.password, database.host, database.port, database.dbname
    );
    let manager = async_bb8_diesel::ConnectionManager::<PgConnection>::new(database_url);
    let mut pool = bb8::Pool::builder().max_size(database.pool_size);

    if test_transaction {
        pool = pool.connection_customizer(Box::new(TestTransaction));
    }

    pool.build(manager)
        .await
        .expect("Failed to create PostgreSQL connection pool")
}

#[allow(clippy::expect_used)]
pub async fn pg_connection(
    pool: &PgPool,
) -> PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>> {
    pool.get()
        .await
        .expect("Couldn't retrieve PostgreSQL connection")
}
