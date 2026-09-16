//! The storage layer: the one place that holds a connection to the observability database.
//!
//! [`crate::core`] asks [`StorageInterface`] for what it needs in domain types and never sees a
//! connection or a diesel row. Mirrors `router/src/db.rs`: each table declares its operations as an
//! interface trait in its own module, [`StorageInterface`] combines them, and every store
//! implements all of them. [`Store`] is the Postgres implementation; another store — a mock, or a
//! wrapper that adds behaviour around [`Store`] — implements the same traits and is chosen in
//! [`crate::state::AppState::new`], with nothing in `core` changing.
//!
//! Adding a table means adding its interface, implementing it for every store, and adding it to
//! the bounds of [`StorageInterface`].
//!
//! Much thinner than the router's storage layer, because almost none of what that layer does
//! applies here: there is one database, no tenants, no replica, no Redis-backed storage scheme and
//! no encrypted columns.

#[path = "alert_manager/mod.rs"]
pub mod alert_manager;
pub mod alerts_info;

use std::{sync::Arc, time::Duration};

use common_utils::external_service::NoOpEventEmitter;
use diesel_models::{
    errors::{DatabaseError, TransactionError},
    DatabaseConnectionWithContext, DejaPgConnection, StorageResult,
};
use error_stack::ResultExt;
use hyperswitch_masking::PeekInterface;

use crate::{errors::ConfigurationError, settings::Database};

/// Every storage operation this service performs.
///
/// Held by [`crate::state::AppState`] as `Arc<dyn StorageInterface>`: one store shared by every
/// worker, so a store never needs to be cloneable itself.
pub trait StorageInterface:
    Send
    + Sync
    + alerts_info::AlertsInfoInterface
    + alert_manager::alert_dicts::db::AlertsDictsInterface
{
}

impl StorageInterface for Store {}

type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<DejaPgConnection>>;

/// The observability database, behind a connection pool.
///
/// Cheap to clone: the pool is shared, not copied.
#[derive(Clone)]
pub struct Store {
    pool: PgPool,
}

impl Store {
    /// Build the pool, opening `min_idle_pool_size` connections before returning.
    ///
    /// With at least one idle connection configured an unreachable database fails here, at boot.
    pub async fn new(database: &Database) -> Result<Self, ConfigurationError> {
        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            database.username,
            database.password.peek(),
            database.host,
            database.port,
            database.dbname
        );

        let pool = bb8::Pool::builder()
            .max_size(database.pool_size)
            .min_idle(Some(database.min_idle_pool_size))
            .connection_timeout(Duration::from_secs(database.connection_timeout))
            .build(async_bb8_diesel::ConnectionManager::new(database_url))
            .await
            // The error is not attached: a connection failure can echo the URL, password included.
            .map_err(|_| {
                ConfigurationError::ConfigParsingError(format!(
                    "failed to connect to the observability database at {}:{}/{}",
                    database.host, database.port, database.dbname
                ))
            })?;

        Ok(Self { pool })
    }

    /// Lease a connection. Returned to the pool when dropped.
    ///
    /// No request id and no event emitter: this service does not report its database calls as
    /// external service calls.
    pub(crate) async fn connection(&self) -> StorageResult<DatabaseConnectionWithContext<'_>> {
        let connection = self
            .pool
            .get()
            .await
            .change_context(DatabaseError::DatabaseConnectionError)?;

        Ok(DatabaseConnectionWithContext::new(
            connection,
            None,
            Arc::new(NoOpEventEmitter),
        ))
    }
}
