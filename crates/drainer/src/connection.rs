use common_utils::DbConnectionParams;
use diesel_models::DejaPgConnection;
use storage_impl::database::store::{DatabaseConnectionWithContext, LeaseTarget, PgPool};

use crate::{settings::Database, Settings};

#[allow(clippy::expect_used)]
pub async fn redis_connection(conf: &Settings) -> redis_interface::RedisConnectionPool {
    redis_interface::RedisConnectionPool::new_without_event_emitter(&conf.redis)
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
    schema: &str,
) -> PgPool {
    let database_url = database.get_database_url(schema);
    let manager = async_bb8_diesel::ConnectionManager::<DejaPgConnection>::new(database_url);
    let pool = bb8::Pool::builder()
        .max_size(database.pool_size)
        .connection_timeout(std::time::Duration::from_secs(database.connection_timeout));

    let raw_pool = pool
        .build(manager)
        .await
        .expect("Failed to create PostgreSQL connection pool");

    PgPool::new_without_event_emitter(raw_pool)
}

#[allow(clippy::expect_used)]
pub async fn pg_connection<'a>(pool: &'a PgPool) -> DatabaseConnectionWithContext<'a> {
    let target = LeaseTarget::DRAINER;
    let (connection, lease_time) = pool
        .lease(target)
        .await
        .expect("Couldn't retrieve PostgreSQL connection");

    // The drainer serves no API request, so its queries carry no request id and emit no external
    // service call events. The lease time is still recorded: the drainer shares Postgres with the
    // router, so its own pool waits are worth seeing.
    DatabaseConnectionWithContext::new(
        connection,
        None,
        pool.event_emitter.clone(),
        target,
        lease_time,
    )
}
