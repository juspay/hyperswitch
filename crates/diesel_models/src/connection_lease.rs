//! Attribution for Postgres connection pool leases.
//!
//! [`DATABASE_CALL_TIME`][crate::metrics] starts its timer once a connection is already
//! leased, so the time a caller spends queued for a pool slot is invisible to it: under
//! contention the queries look fast while the request looks slow, and the difference lands
//! in no metric at all. This module records the lease itself.
//!
//! A lease is recorded exactly once, by whichever of these happens first:
//!
//! - the first tracked query run on it, which attributes the lease to that query's table —
//!   the common case, since almost every lease serves exactly one query;
//! - [`Drop`][crate::DatabaseConnectionWithContext] of the connection, if no tracked query
//!   ever ran, recorded under `table = "unattributed"`. The KV path leases a write
//!   connection purely to serialize a drainer query and runs nothing on it, so without
//!   this those leases would vanish from the metric;
//! - the lease site, when the lease failed, under `table = "none"` — no connection exists
//!   to carry it.
//!
//! A lease serving several queries is therefore counted once, against the first table it
//! served, rather than once per query.
//!
//! ## Reading the `table` attribute
//!
//! It says which code paths are *waiting*, not which are *causing* the wait. Pool
//! exhaustion comes from aggregate pressure, so a table ranking high here is a victim at
//! least as often as a culprit.

use std::time::Duration;

use crate::metrics;

/// The logical database a pool belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Database {
    /// The transactional database, configured as `master_database` / `replica_database`.
    Primary,
    /// The merchant-account database, configured as `accounts_database`.
    Accounts,
}

impl Database {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Accounts => "accounts",
        }
    }
}

/// Which pool within a [`Database`] a lease came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolKind {
    /// The writable primary. Reads land here too whenever `oltp` is enabled, which
    /// includes the default build.
    Master,
    /// A read replica.
    Replica,
}

impl PoolKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Replica => "replica",
        }
    }
}

/// What a caller leased a connection for.
///
/// This is the caller's intent, decided before any query exists — which is why it is not
/// one of the `DatabaseOperation` enums in this crate. Those describe a statement that has
/// already been built (`Filter`, `Insert`, `UpdateOne`, …); a lease predates all of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Leased to read.
    Read,
    /// Leased to write.
    Write,
    /// Leased without the caller saying which, as the drainer does.
    Unspecified,
}

impl Access {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Unspecified => "unspecified",
        }
    }
}

/// Which pool a lease came from, and what the caller wanted it for.
///
/// Carried as a struct rather than loose arguments so that call sites cannot transpose two
/// parameters, and so the values live in exactly one place. Use the associated constants;
/// they enumerate every combination that exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaseTarget {
    database: Database,
    pool: PoolKind,
    access: Access,
}

impl LeaseTarget {
    /// A read served by the primary master pool — what `pg_connection_read` does whenever
    /// `oltp` is enabled, which includes the default build, so these reads contend with
    /// writes for the same connections.
    pub const PRIMARY_MASTER_READ: Self = Self {
        database: Database::Primary,
        pool: PoolKind::Master,
        access: Access::Read,
    };
    /// A read served by the primary replica pool.
    pub const PRIMARY_REPLICA_READ: Self = Self {
        database: Database::Primary,
        pool: PoolKind::Replica,
        access: Access::Read,
    };
    /// A write, which always goes to the primary master pool.
    pub const PRIMARY_MASTER_WRITE: Self = Self {
        database: Database::Primary,
        pool: PoolKind::Master,
        access: Access::Write,
    };
    /// A read served by the accounts master pool.
    pub const ACCOUNTS_MASTER_READ: Self = Self {
        database: Database::Accounts,
        pool: PoolKind::Master,
        access: Access::Read,
    };
    /// A read served by the accounts replica pool.
    pub const ACCOUNTS_REPLICA_READ: Self = Self {
        database: Database::Accounts,
        pool: PoolKind::Replica,
        access: Access::Read,
    };
    /// A write, which always goes to the accounts master pool.
    pub const ACCOUNTS_MASTER_WRITE: Self = Self {
        database: Database::Accounts,
        pool: PoolKind::Master,
        access: Access::Write,
    };
    /// The drainer's single pool. It leases one connection for whatever its handler needs
    /// next and does not distinguish read from write at the lease.
    pub const DRAINER: Self = Self {
        database: Database::Primary,
        pool: PoolKind::Master,
        access: Access::Unspecified,
    };
}

/// What became of a lease attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// A connection was leased.
    Acquired,
    /// The pool had no slot free within `connection_timeout`. This is the saturation
    /// signal: the caller waited the full timeout and got nothing.
    TimedOut,
    /// A connection could not be established.
    Error,
}

impl Outcome {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Acquired => "acquired",
            Self::TimedOut => "timed_out",
            Self::Error => "error",
        }
    }
}

/// Records one lease attempt.
///
/// The duration is recorded whatever the outcome, so a lease that waited out
/// `connection_timeout` shows the time it really spent while staying separable from one
/// that succeeded.
pub fn record(target: LeaseTarget, outcome: Outcome, table: &'static str, elapsed: Duration) {
    metrics::DATABASE_CONNECTION_LEASE_TIME_MS.record(
        elapsed.as_secs_f64() * 1_000.0,
        router_env::metric_attributes!(
            ("db", target.database.as_str()),
            ("pool", target.pool.as_str()),
            ("operation", target.access.as_str()),
            ("outcome", outcome.as_str()),
            ("table", table),
        ),
    );
}
