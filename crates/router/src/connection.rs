use async_bb8_diesel::{AsyncConnection, ConnectionError};
use bb8::{CustomizeConnection, PooledConnection};
use diesel::PgConnection;
use error_stack::{IntoReport, ResultExt};
#[cfg(feature = "kms")]
use external_services::kms;
#[cfg(feature = "kms")]
use external_services::kms::decrypt::KmsDecrypt;
#[cfg(not(feature = "kms"))]
use masking::PeekInterface;

use crate::{configs::settings::Database, errors};

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;

pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

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

#[allow(clippy::expect_used)]
pub async fn redis_connection(
    conf: &crate::configs::settings::Settings,
) -> redis_interface::RedisConnectionPool {
    redis_interface::RedisConnectionPool::new(&conf.redis)
        .await
        .expect("Failed to create Redis Connection Pool")
}

#[allow(clippy::expect_used)]
pub async fn diesel_make_pg_pool(
    database: &Database,
    test_transaction: bool,
    #[cfg(feature = "kms")] kms_client: &kms::KmsClient,
) -> PgPool {
    #[cfg(feature = "kms")]
    let password = database
        .password
        .clone()
        .decrypt_inner(kms_client)
        .await
        .expect("Failed to KMS decrypt database password");

    #[cfg(not(feature = "kms"))]
    let password = &database.password.peek();

    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        database.username, password, database.host, database.port, database.dbname
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

pub async fn pg_connection_read(
    store: &crate::services::Store,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    errors::StorageError,
> {
    // If only OLAP is enabled get replica pool.
    #[cfg(all(feature = "olap", not(feature = "oltp")))]
    let pool = &store.diesel_store.replica_pool;

    // If either one of these are true we need to get master pool.
    //  1. Only OLTP is enabled.
    //  2. Both OLAP and OLTP is enabled.
    //  3. Both OLAP and OLTP is disabled.
    #[cfg(any(
        all(not(feature = "olap"), feature = "oltp"),
        all(feature = "olap", feature = "oltp"),
        all(not(feature = "olap"), not(feature = "oltp"))
    ))]
    let pool = &store.diesel_store.master_pool;

    pool.get()
        .await
        .into_report()
        .change_context(errors::StorageError::DatabaseConnectionError)
}

pub async fn pg_connection_write(
    store: &crate::services::Store,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = &store.diesel_store.master_pool;

    pool.get()
        .await
        .into_report()
        .change_context(errors::StorageError::DatabaseConnectionError)
}
