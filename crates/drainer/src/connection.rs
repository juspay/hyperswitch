use bb8::PooledConnection;
use diesel::PgConnection;
#[cfg(feature = "kms")]
use external_services::kms;

use crate::settings::Database;

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;

#[allow(clippy::expect_used)]
pub async fn redis_connection(
    conf: &crate::settings::Settings,
) -> redis_interface::RedisConnectionPool {
    redis_interface::RedisConnectionPool::new(&conf.redis)
        .await
        .expect("Failed to create Redis connection Pool")
}

#[allow(clippy::expect_used)]
pub async fn diesel_make_pg_pool(
    database: &Database,
    _test_transaction: bool,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
) -> PgPool {
    #[cfg(feature = "kms")]
    let password = kms::get_kms_client(kms_config)
        .await
        .decrypt(&database.kms_encrypted_password)
        .await
        .expect("Failed to KMS decrypt database password");

    #[cfg(not(feature = "kms"))]
    let password = &database.password;

    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        database.username, password, database.host, database.port, database.dbname
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
