use bb8::PooledConnection;
use diesel::PgConnection;
use external_services::hashicorp_vault::{self, decrypt::VaultFetch, Kv2};
#[cfg(feature = "kms")]
use external_services::kms::{self, decrypt::KmsDecrypt};
use masking::ExposeInterface;
#[cfg(not(feature = "kms"))]
use masking::PeekInterface;

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

// TODO: use stores defined in storage_impl instead
/// # Panics
///
/// Will panic if could not create a db pool
#[allow(clippy::expect_used)]
pub async fn diesel_make_pg_pool(
    database: &Database,
    _test_transaction: bool,
    #[cfg(feature = "kms")] kms_client: &'static kms::KmsClient,
    #[cfg(feature = "hashicorp-vault")] hashicorp_client: &'static hashicorp_vault::HashiCorpVault,
) -> PgPool {
    #[cfg(feature = "hashicorp-vault")]
    let password = database
        .password
        .clone()
        .decrypt_inner::<Kv2>(hashicorp_client)
        .await
        .expect("Failed while fetching db password")
        .expose();

    #[cfg(feature = "kms")]
    let password = database
        .password
        .decrypt_inner(kms_client)
        .await
        .expect("Failed to decrypt password");

    #[cfg(all(not(feature = "kms"), not(feature = "hashicorp-vault")))]
    let password = &database.password.peek();

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
