//! Per-session tracking for the duplicate database-read metric.
//!
//! Store-labelled ledger views share one set of reads per API session. Reads are identified by their SQL
//! statement including bind values, so different parameters stay distinct.

use std::{
    collections::{BTreeMap, HashMap},
    panic::Location,
    sync::{Arc, Mutex},
};

/// Session store issuing a tracked database read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadStore {
    /// Main application store.
    Main,
    /// Accounts store.
    Accounts,
    /// Global store.
    Global,
}

impl ReadStore {
    /// Bounded store label used by the duplicate-read metric.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Accounts => "accounts",
            Self::Global => "global",
        }
    }
}

/// Single reads are promoted into `duplicates` on their second occurrence.
/// Both collections share a lock so concurrent reads and snapshots stay consistent.
#[derive(Debug, Default)]
struct ReadLedgerState {
    single_reads: HashMap<String, Option<Location<'static>>>,
    duplicates: BTreeMap<String, Vec<Option<Location<'static>>>>,
}

impl ReadLedgerState {
    fn record_read(&mut self, statement: &str, location: Option<Location<'static>>) -> bool {
        if let Some(locations) = self.duplicates.get_mut(statement) {
            locations.push(location);
            true
        } else if let Some((statement, first_location)) = self.single_reads.remove_entry(statement)
        {
            self.duplicates
                .insert(statement, vec![first_location, location]);
            true
        } else {
            self.single_reads.insert(statement.to_owned(), location);
            false
        }
    }

    fn duplicate_reads(&self) -> Vec<(String, Vec<Option<Location<'static>>>)> {
        self.duplicates
            .iter()
            .map(|(statement, locations)| (statement.clone(), locations.clone()))
            .collect()
    }
}

/// Statements already read in an API session, shared across its stores.
#[derive(Debug, Clone)]
pub struct ReadLedger {
    flow: String,
    store: ReadStore,
    // Keep complete statements rather than fingerprints to avoid treating hash
    // collisions as duplicate reads. Bind values are never emitted as metrics.
    state: Arc<Mutex<ReadLedgerState>>,
}

impl ReadLedger {
    /// A ledger for a fresh API session.
    pub fn new(flow: String) -> Self {
        Self {
            flow,
            store: ReadStore::Main,
            state: Arc::new(Mutex::new(ReadLedgerState::default())),
        }
    }

    /// Label reads from another store while retaining this session's shared set.
    pub fn for_store(&self, store: ReadStore) -> Self {
        Self {
            store,
            ..self.clone()
        }
    }

    /// API flow shared by all stores in this session.
    pub fn flow(&self) -> &str {
        &self.flow
    }

    /// Store issuing the current read, rather than the store of the first read.
    pub fn store(&self) -> ReadStore {
        self.store
    }

    /// Record a read with its optional captured origin, returning whether the session
    /// already read the same SQL with the same bind values. A second read promotes
    /// the statement into the duplicate collection; later reads append their origins.
    pub fn record_read(&self, statement: &str, location: Option<Location<'static>>) -> bool {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .record_read(statement, location)
    }

    /// Snapshot duplicate statements in SQL order, with locations in recording order.
    pub fn duplicate_reads(&self) -> Vec<(String, Vec<Option<Location<'static>>>)> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .duplicate_reads()
    }
}

#[cfg(test)]
mod test;
