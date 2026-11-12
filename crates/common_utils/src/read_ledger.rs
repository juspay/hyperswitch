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
mod tests {
    use super::*;

    const PI: &str = "SELECT .. FROM payment_intent WHERE payment_id = $1 -- binds: [pay_1]";
    const OTHER: &str = "SELECT .. FROM payment_intent WHERE payment_id = $1 -- binds: [pay_2]";

    #[track_caller]
    fn record_read(ledger: &ReadLedger, statement: &str) -> bool {
        ledger.record_read(statement, Some(*Location::caller()))
    }

    #[test]
    fn only_reads_after_the_first_are_duplicates() {
        let ledger = ReadLedger::new("PaymentsConfirm".to_owned());
        assert!(!record_read(&ledger, PI));
        for _ in 0..20 {
            assert!(record_read(&ledger, PI));
        }
    }

    #[test]
    fn records_every_location_for_a_statement_in_order() {
        let ledger = ReadLedger::new("PaymentsConfirm".to_owned());
        let first = *Location::caller();
        let second = *Location::caller();

        assert!(!ledger.record_read(PI, Some(first)));
        assert!(ledger.duplicate_reads().is_empty());
        assert!(!ledger.record_read(OTHER, Some(first)));
        assert!(ledger.record_read(PI, Some(second)));
        assert!(ledger.record_read(PI, None));

        assert_eq!(
            ledger.duplicate_reads(),
            vec![(PI.to_owned(), vec![Some(first), Some(second), None])]
        );
    }

    #[test]
    fn duplicate_snapshots_are_sorted_and_independent() {
        let ledger = ReadLedger::new("PaymentsConfirm".to_owned());
        assert!(!ledger.record_read(OTHER, None));
        assert!(ledger.record_read(OTHER, None));
        let snapshot = ledger.duplicate_reads();

        assert!(!ledger.record_read(PI, None));
        assert!(ledger.record_read(PI, None));
        assert!(ledger.record_read(OTHER, None));
        assert!(!ledger.record_read("SELECT 1", None));

        assert_eq!(snapshot, vec![(OTHER.to_owned(), vec![None, None])]);
        assert_eq!(
            ledger.duplicate_reads(),
            vec![
                (PI.to_owned(), vec![None, None]),
                (OTHER.to_owned(), vec![None, None, None]),
            ]
        );
    }

    #[test]
    fn different_binds_are_different_records() {
        let ledger = ReadLedger::new("PaymentsConfirm".to_owned());
        assert!(!record_read(&ledger, PI));
        assert!(!record_read(&ledger, OTHER));
        assert!(record_read(&ledger, OTHER));
        assert!(record_read(&ledger, PI));
    }

    #[test]
    fn sessions_do_not_share_reads() {
        let first = ReadLedger::new("PaymentsConfirm".to_owned());
        let second = ReadLedger::new("PaymentsConfirm".to_owned());
        assert!(!record_read(&first, PI));
        assert!(!record_read(&second, PI));
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

        assert!(!record_read(&main, PI));
        assert!(record_read(&accounts, PI));
        assert!(record_read(&global, PI));
        assert!(!record_read(&accounts, OTHER));
        assert!(record_read(&main, OTHER));
        assert_eq!(main.duplicate_reads(), accounts.duplicate_reads());
        assert_eq!(main.duplicate_reads(), global.duplicate_reads());
        assert_eq!(main.duplicate_reads().len(), 2);
    }

    #[test]
    fn different_statements_are_different_reads() {
        let ledger = ReadLedger::new("PaymentsConfirm".to_owned());
        assert!(!record_read(&ledger, "SELECT 1908"));
        assert!(!record_read(&ledger, "SELECT 3600"));
        assert!(record_read(&ledger, "SELECT 1908"));
        assert!(record_read(&ledger, "SELECT 3600"));
    }

    #[test]
    fn distinct_statements_are_all_tracked() {
        let ledger = ReadLedger::new("PaymentsConfirm".to_owned());
        for index in 0..5_000 {
            assert!(!record_read(&ledger, &format!("SELECT {index}")));
        }
        assert!(record_read(&ledger, "SELECT 0"));
        assert!(record_read(&ledger, "SELECT 4999"));
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
                    record_read(&ledger, PI)
                })
            })
            .collect::<Vec<_>>();
        let repeats = threads
            .into_iter()
            .map(|thread| usize::from(thread.join().expect("read thread")))
            .sum::<usize>();
        assert_eq!(repeats, 7);
        let duplicates = ledger.duplicate_reads();
        assert_eq!(duplicates.len(), 1);
        let (statement, locations) = duplicates.first().expect("duplicate read");
        assert_eq!(statement, PI);
        assert_eq!(locations.len(), 8);
    }
}
