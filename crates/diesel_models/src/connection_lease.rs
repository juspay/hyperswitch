//! Attribution for Postgres connection pool leases.
//!
//! [`DATABASE_CALL_TIME`][crate::metrics] times a connection only after it's leased, so time
//! spent queued for a pool slot is invisible to it. This module records the lease itself.
//!
//! A lease is recorded once, by whichever happens first:
//! - the first tracked query run on it, attributed to that query's table;
//! - [`Drop`][crate::DatabaseConnectionWithContext], under `table = "unattributed"`, if no
//!   query ran — e.g. the KV path leases a write connection purely to serialize a drainer
//!   query;
//! - the lease site itself, under `table = "none"`, if the lease failed.
//!
//! ## Reading the `table` attribute
//!
//! It names which code paths are *waiting*, not which are *causing* the wait — pool
//! exhaustion is aggregate pressure, so a table ranking high here is as often a victim as a
//! culprit.

use std::time::Duration;

use crate::metrics;

/// The logical database a pool belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Database {
    /// Configured as `master_database` / `replica_database`.
    Primary,
    /// Configured as `accounts_database`.
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
    /// Also serves reads whenever `oltp` is enabled (the default build).
    Master,
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

/// What a caller leased a connection for — decided before any query exists, so distinct
/// from the `DatabaseOperation` enums (`Filter`, `Insert`, …), which describe an
/// already-built statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    /// As the drainer does.
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
/// A struct rather than loose arguments, so call sites can't transpose two parameters. Use
/// the associated constants — they cover every valid combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaseTarget {
    database: Database,
    pool: PoolKind,
    access: Access,
}

impl LeaseTarget {
    /// `pg_connection_read`'s target whenever `oltp` is enabled (the default build) — these
    /// reads contend with writes for the same connections.
    pub const PRIMARY_MASTER_READ: Self = Self {
        database: Database::Primary,
        pool: PoolKind::Master,
        access: Access::Read,
    };
    pub const PRIMARY_REPLICA_READ: Self = Self {
        database: Database::Primary,
        pool: PoolKind::Replica,
        access: Access::Read,
    };
    pub const PRIMARY_MASTER_WRITE: Self = Self {
        database: Database::Primary,
        pool: PoolKind::Master,
        access: Access::Write,
    };
    pub const ACCOUNTS_MASTER_READ: Self = Self {
        database: Database::Accounts,
        pool: PoolKind::Master,
        access: Access::Read,
    };
    pub const ACCOUNTS_REPLICA_READ: Self = Self {
        database: Database::Accounts,
        pool: PoolKind::Replica,
        access: Access::Read,
    };
    pub const ACCOUNTS_MASTER_WRITE: Self = Self {
        database: Database::Accounts,
        pool: PoolKind::Master,
        access: Access::Write,
    };
    /// The drainer's one pool — reads and writes aren't distinguished at the lease.
    pub const DRAINER: Self = Self {
        database: Database::Primary,
        pool: PoolKind::Master,
        access: Access::Unspecified,
    };
}

/// What became of a lease attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Acquired,
    /// No pool slot freed up within `connection_timeout` — the saturation signal.
    TimedOut,
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

/// Records one lease attempt. The duration is kept even on failure or timeout.
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
