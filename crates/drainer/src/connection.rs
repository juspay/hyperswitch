use common_utils::DbConnectionParams;
use diesel_models::DejaPgConnection;
use storage_impl::database::store::{DatabaseConnectionWithContext, PgPool};

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
    db_pool: storage_impl::database::pool_metrics::DbPool,
    tenant_id: &common_utils::id_type::TenantId,
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

    let pool_metrics = storage_impl::database::pool_metrics::PgPoolMetrics::new(
        raw_pool.clone(),
        db_pool,
        tenant_id,
    );

    PgPool::new(
        raw_pool,
        std::sync::Arc::new(common_utils::external_service::NoOpEventEmitter),
        tenant_id.clone(),
        pool_metrics,
    )
}

#[allow(clippy::expect_used)]
pub async fn pg_connection<'a>(pool: &'a PgPool) -> DatabaseConnectionWithContext<'a> {
    let tenant_id = pool.tenant_id.get_string_repr();

    let connection = storage_impl::metrics::record_db_connection_acquire_duration(
        pool.pg_pool.get(),
        storage_impl::database::pool_metrics::DbPool::Master,
        tenant_id,
    )
    .await
    .expect("Couldn't retrieve PostgreSQL connection");

    // The drainer serves no API request, so its queries carry no request id and emit no external
    // service call events.
    DatabaseConnectionWithContext::new(connection, None, pool.event_emitter.clone())
}
