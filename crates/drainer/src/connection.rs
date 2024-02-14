use bb8::PooledConnection;
use diesel::PgConnection;
use masking::PeekInterface;

use crate::{settings::Database, Settings};

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;

#[allow(clippy::expect_used)]
pub async fn redis_connection(conf: &Settings) -> redis_interface::RedisConnectionPool {
    redis_interface::RedisConnectionPool::new(&conf.redis)
        .await
        .expect("Failed to create Redis connection Pool")
}

// TODO: use stores defined in storage_impl instead
/// # Panics
///
/// Will panic if could not create a db pool
#[allow(clippy::expect_used)]
pub async fn diesel_make_pg_pool(database: &Database, _test_transaction: bool) -> PgPool {
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        database.username,
        database.password.peek(),
        database.host,
        database.port,
        database.dbname
    );
    let manager = async_bb8_diesel::ConnectionManager::<PgConnection>::new(database_url);
    let pool = bb8::Pool::builder()
        .max_size(database.pool_size)
        .connection_timeout(std::time::Duration::from_secs(database.connection_timeout));

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
