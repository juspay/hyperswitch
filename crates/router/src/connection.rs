use bb8::PooledConnection;
use diesel::PgConnection;
use error_stack::ResultExt;
use storage_impl::errors as storage_errors;

use crate::errors;

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;

pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

/// Creates a Redis connection pool for the specified Redis settings
/// # Panics
///
/// Panics if failed to create a redis pool
#[allow(clippy::expect_used)]
pub async fn redis_connection(
    conf: &crate::configs::Settings,
) -> redis_interface::RedisConnectionPool {
    redis_interface::RedisConnectionPool::new(&conf.redis)
        .await
        .expect("Failed to create Redis Connection Pool")
}
