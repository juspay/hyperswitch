//! Observable instruments for database pool state.

use common_utils::id_type::TenantId;
use router_env::opentelemetry::metrics::{ObservableCounter, ObservableGauge};

use super::store::RawPgPool;

#[derive(Debug, Clone, Copy)]
pub enum DbPool {
    Master,
    Replica,
    AccountsMaster,
    AccountsReplica,
}

impl DbPool {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Replica => "replica",
            Self::AccountsMaster => "accounts_master",
            Self::AccountsReplica => "accounts_replica",
        }
    }
}

// The fields are underscore-prefixed because the handles are never read directly.
// They exist only to keep the callbacks alive.
#[derive(Clone)]
pub struct PgPoolMetrics {
    _pool_size: ObservableGauge<u64>,
    _pool_available: ObservableGauge<u64>,
    _pool_waiting: ObservableGauge<u64>,
    _created_connections: ObservableCounter<u64>,
    _closed_connections: ObservableCounter<u64>,
}

impl std::fmt::Debug for PgPoolMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgPoolMetrics").finish_non_exhaustive()
    }
}

impl PgPoolMetrics {
    pub fn new(pool: RawPgPool, db_pool: DbPool, tenant_id: &TenantId) -> Self {
        let _pool_size = Self::build_pool_gauge(
            "database.pool.size",
            "Total number of connections in the database pool",
            "{connection}",
            db_pool,
            pool.clone(),
            tenant_id,
            |p| u64::from(p.state().connections),
        );

        let _pool_available = Self::build_pool_gauge(
            "database.pool.available",
            "Number of idle connections in the database pool",
            "{connection}",
            db_pool,
            pool.clone(),
            tenant_id,
            |p| u64::from(p.state().idle_connections),
        );

        let _pool_waiting = Self::build_pool_gauge(
            "database.pool.waiting",
            "Number of callers waiting for a database connection",
            "{connection}",
            db_pool,
            pool.clone(),
            tenant_id,
            |p| p.state().statistics.pending_gets(),
        );

        let _created_connections = {
            let pool_clone = pool.clone();
            let tenant_id_clone = tenant_id.to_owned();
            crate::metrics::GLOBAL_METER
                .u64_observable_counter("database.pool.created")
                .with_description("Total database connections created")
                .with_unit("{connection}")
                .with_callback(move |observer| {
                    let stats = pool_clone.state().statistics;
                    observer.observe(
                        stats.connections_created,
                        router_env::metric_attributes!(
                            ("pool", db_pool.as_str()),
                            ("tenant_id", tenant_id_clone.get_string_repr().to_owned())
                        ),
                    );
                })
                .build()
        };

        let _closed_connections = {
            let tenant_id_clone = tenant_id.to_owned();
            crate::metrics::GLOBAL_METER
                .u64_observable_counter("database.pool.closed")
                .with_description("Total database connections closed")
                .with_unit("{connection}")
                .with_callback(move |observer| {
                    let stats = pool.state().statistics;
                    let tenant_id_str = tenant_id_clone.get_string_repr().to_owned();
                    for (value, reason) in [
                        (stats.connections_closed_broken, "broken"),
                        (stats.connections_closed_invalid, "invalid"),
                        (stats.connections_closed_max_lifetime, "max_lifetime"),
                        (stats.connections_closed_idle_timeout, "idle_timeout"),
                    ] {
                        observer.observe(
                            value,
                            router_env::metric_attributes!(
                                ("pool", db_pool.as_str()),
                                ("tenant_id", tenant_id_str.clone()),
                                ("reason", reason)
                            ),
                        );
                    }
                })
                .build()
        };

        Self {
            _pool_size,
            _pool_available,
            _pool_waiting,
            _created_connections,
            _closed_connections,
        }
    }

    fn build_pool_gauge(
        name: &'static str,
        description: &'static str,
        unit: &'static str,
        db_pool: DbPool,
        pool: RawPgPool,
        tenant_id: &TenantId,
        read: impl Fn(&RawPgPool) -> u64 + Send + Sync + 'static,
    ) -> ObservableGauge<u64> {
        let tenant_id = tenant_id.to_owned();
        crate::metrics::GLOBAL_METER
            .u64_observable_gauge(name)
            .with_description(description)
            .with_unit(unit)
            .with_callback(move |observer| {
                observer.observe(
                    read(&pool),
                    router_env::metric_attributes!(
                        ("pool", db_pool.as_str()),
                        ("tenant_id", tenant_id.get_string_repr().to_owned())
                    ),
                );
            })
            .build()
    }
}
