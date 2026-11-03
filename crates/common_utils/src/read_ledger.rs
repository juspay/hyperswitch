//! Per-session tracking for the duplicate database-read metric.
//!
//! Store-labelled ledger views share one set of reads per API session. Reads are identified by their SQL
//! statement including bind values, so different parameters stay distinct.

use std::{
    collections::HashSet,
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

/// Statements already read in an API session, shared across its stores.
#[derive(Debug, Clone)]
pub struct ReadLedger {
    flow: String,
    store: ReadStore,
    // Keep complete statements rather than fingerprints to avoid treating hash
    // collisions as duplicate reads. Bind values are never emitted as metrics.
    reads: Arc<Mutex<HashSet<String>>>,
}

impl ReadLedger {
    /// A ledger for a fresh API session.
    pub fn new(flow: String) -> Self {
        Self {
            flow,
            store: ReadStore::Main,
            reads: Arc::new(Mutex::new(HashSet::new())),
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

    /// Record a read, returning whether the session already read the same SQL
    /// with the same bind values. Only the first read allocates a stored key.
    pub fn record_read(&self, statement: &str) -> bool {
        let mut reads = self
            .reads
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if reads.contains(statement) {
            true
        } else {
            reads.insert(statement.to_owned());
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PI: &str = "SELECT .. FROM payment_intent WHERE payment_id = $1 -- binds: [pay_1]";
    const OTHER: &str = "SELECT .. FROM payment_intent WHERE payment_id = $1 -- binds: [pay_2]";

    #[test]
    fn only_reads_after_the_first_are_duplicates() {
        let ledger = ReadLedger::new("PaymentsConfirm".to_owned());
        assert!(!ledger.record_read(PI));
        for _ in 0..20 {
            assert!(ledger.record_read(PI));
        }
    }

    #[test]
    fn different_binds_are_different_records() {
        let ledger = ReadLedger::new("PaymentsConfirm".to_owned());
        assert!(!ledger.record_read(PI));
        assert!(!ledger.record_read(OTHER));
        assert!(ledger.record_read(OTHER));
        assert!(ledger.record_read(PI));
    }

    #[test]
    fn sessions_do_not_share_reads() {
        let first = ReadLedger::new("PaymentsConfirm".to_owned());
        let second = ReadLedger::new("PaymentsConfirm".to_owned());
        assert!(!first.record_read(PI));
        assert!(!second.record_read(PI));
    }

    #[test]
    fn store_labels_share_reads_and_preserve_the_session_flow() {
        let main = ReadLedger::new("PaymentsConfirm".to_owned());
        let accounts = main.for_store(ReadStore::Accounts);
        let global = main.for_store(ReadStore::Global);

        assert_eq!(main.store().as_str(), "main");
        assert_eq!(accounts.store().as_str(), "accounts");
        assert_eq!(global.store().as_str(), "global");
        for ledger in [&main, &accounts, &global] {
            assert_eq!(ledger.flow(), "PaymentsConfirm");
        }

        assert!(!main.record_read(PI));
        assert!(accounts.record_read(PI));
        assert!(global.record_read(PI));
        assert!(!accounts.record_read(OTHER));
        assert!(main.record_read(OTHER));
    }

    #[test]
    fn different_statements_are_different_reads() {
        let ledger = ReadLedger::new("PaymentsConfirm".to_owned());
        assert!(!ledger.record_read("SELECT 1908"));
        assert!(!ledger.record_read("SELECT 3600"));
        assert!(ledger.record_read("SELECT 1908"));
        assert!(ledger.record_read("SELECT 3600"));
    }

    #[test]
    fn distinct_statements_are_all_tracked() {
        let ledger = ReadLedger::new("PaymentsConfirm".to_owned());
        for index in 0..5_000 {
            assert!(!ledger.record_read(&format!("SELECT {index}")));
        }
        assert!(ledger.record_read("SELECT 0"));
        assert!(ledger.record_read("SELECT 4999"));
    }

    #[test]
    fn shared_ledger_counts_each_concurrent_repeat_once() {
        let ledger = Arc::new(ReadLedger::new("PaymentsConfirm".to_owned()));
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let threads = (0..8)
            .map(|_| {
                let ledger = Arc::clone(&ledger);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    ledger.record_read(PI)
                })
            })
            .collect::<Vec<_>>();
        let repeats = threads
            .into_iter()
            .map(|thread| usize::from(thread.join().expect("read thread")))
            .sum::<usize>();
        assert_eq!(repeats, 7);
    }
}
