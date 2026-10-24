//! Which records an API session has already read.
//!
//! One ledger per API session, shared by every store it touches, so a record
//! fetched through two of them is one record. `track_database_call` records
//! reads here; the session drains them at the end with
//! [`ReadLedger::take_duplicates`] and logs one line. The ledger logs nothing
//! itself -- `router_env` is optional for this crate.
//!
//! A record is identified by the statement that selected it, binds included,
//! so that text is held for the session, though never reported.
//!
//! Only reads are tracked: the row-returning update builders hand back the
//! updated row, so a read after a write is still one the flow did not need.

use std::{collections::HashMap, sync::Mutex};

/// Call sites named per repeated record; repeats past this are still counted.
/// A limit on noise, not memory: a record read in a loop repeats from the same
/// call site, so the ninth copy of that path says nothing the eighth did.
const ORIGINS_PER_RECORD: usize = 8;

/// Raw stack frames retained until duplicate reporting in debug builds.
#[cfg(debug_assertions)]
pub type ReadOrigin = Vec<backtrace::BacktraceFrame>;
/// Release builds do not capture stack frames.
#[cfg(not(debug_assertions))]
pub type ReadOrigin = ();

fn resolve_origin(origin: ReadOrigin) -> Option<String> {
    #[cfg(debug_assertions)]
    {
        crate::read_origin::resolve(origin)
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = origin;
        None
    }
}

/// A record this session read more than once.
#[derive(Debug, serde::Serialize)]
pub struct DuplicateRead {
    /// Table the record belongs to.
    pub table: &'static str,
    /// How many reads of it were repeats of the first.
    pub repeats: usize,
    /// Where it was first read, as a compact chain of source locations.
    /// `None` in release builds, or when a debug build lacks source locations.
    pub first_read_from: Option<String>,
    /// Where each repeat was issued, up to [`ORIGINS_PER_RECORD`].
    /// `None` entries preserve repeats whose origins were unavailable, including
    /// in release builds where backtraces are not captured.
    pub re_read_from: Vec<Option<String>>,
}

#[derive(Debug)]
struct Record {
    table: &'static str,
    /// Absent in release builds; debug source locations are resolved at reporting.
    first_origin: Option<ReadOrigin>,
    repeats: usize,
    /// Keep a `None` placeholder for each retained repeat without an origin.
    repeat_origins: Vec<Option<ReadOrigin>>,
}

/// Per-session record of what has already been read.
#[derive(Debug, Default)]
pub struct ReadLedger {
    /// Statement to what is known about reads of it.
    ///
    /// Keyed by the statement, not a digest of it: two records under one key
    /// would be reported as repeats of each other, and a 64-bit `FxHash` of
    /// 5000 short statements put 248 colliding pairs among them. Keys carry
    /// bind values, so the map is a plain `HashMap`: a per-map random seed
    /// rather than a fixed one, and at this size -- one entry per round trip
    /// the request already paid for -- the cost of that is nothing.
    reads: Mutex<HashMap<String, Record>>,
}

impl ReadLedger {
    /// A ledger for a fresh API session.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a read of whatever `statement` selects, returning whether this
    /// session had already run it -- same SQL, same binds, so the same rows.
    /// `origin` is where the read was issued, kept so the report can name call
    /// sites rather than just counts.
    pub fn record_read(
        &self,
        table: &'static str,
        statement: &str,
        origin: Option<ReadOrigin>,
    ) -> bool {
        let mut reads = self.reads();

        // Borrowed lookup: only a statement's first read allocates.
        if let Some(record) = reads.get_mut(statement) {
            record.repeats += 1;
            if record.repeat_origins.len() < ORIGINS_PER_RECORD {
                record.repeat_origins.push(origin);
            }
            true
        } else {
            // Uncapped: one entry per distinct statement, and a distinct statement
            // is a round trip the request already paid for. A cap would only stop
            // detecting on the worst sessions, and every entry is a different
            // record, so nothing it dropped would be redundant.
            reads.insert(
                statement.to_owned(),
                Record {
                    table,
                    first_origin: origin,
                    repeats: 0,
                    repeat_origins: Vec::new(),
                },
            );
            false
        }
    }

    /// Take the session's duplicate reads, worst first, and forget them.
    ///
    /// The session logs these as one line rather than warning per duplicate: a
    /// flow that re-reads an intent four times is one thing to fix. Taking
    /// rather than borrowing means a second flush cannot double-report.
    /// Resolves retained origins synchronously; call on a blocking worker.
    pub fn take_duplicates(&self) -> Vec<DuplicateRead> {
        // Release the ledger lock before any potentially expensive resolution.
        let reads = std::mem::take(&mut *self.reads());
        let mut duplicates = reads
            .into_values()
            .filter(|record| record.repeats > 0)
            .map(|record| DuplicateRead {
                table: record.table,
                repeats: record.repeats,
                first_read_from: record.first_origin.and_then(resolve_origin),
                re_read_from: record
                    .repeat_origins
                    .into_iter()
                    .map(|origin| origin.and_then(resolve_origin))
                    .collect(),
            })
            .collect::<Vec<_>>();
        duplicates.sort_by_key(|duplicate| std::cmp::Reverse(duplicate.repeats));
        duplicates
    }

    /// A diagnostic is not worth failing a payment over, so a poisoned lock is
    /// recovered rather than propagated.
    fn reads(&self) -> std::sync::MutexGuard<'_, HashMap<String, Record>> {
        self.reads
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PI: &str = "SELECT .. FROM payment_intent WHERE payment_id = $1 -- binds: [pay_1]";
    const OTHER: &str = "SELECT .. FROM payment_intent WHERE payment_id = $1 -- binds: [pay_2]";

    #[test]
    fn a_single_read_is_nothing_to_report() {
        let ledger = ReadLedger::new();
        assert!(!ledger.record_read("payment_intent", PI, None));
        assert!(ledger.take_duplicates().is_empty());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn repeats_preserve_the_original_frames_across_threads() {
        let first = crate::read_origin::capture();
        let repeat = crate::read_origin::capture();
        let expected_first = resolve_origin(first.clone());
        let expected_repeat = resolve_origin(repeat.clone());
        assert!(expected_first.is_some());
        assert!(expected_repeat.is_some());
        let ledger = ReadLedger::new();
        assert!(!ledger.record_read("payment_intent", PI, Some(first)));
        assert!(ledger.record_read("payment_intent", PI, Some(repeat)));
        let duplicates = std::thread::spawn(move || ledger.take_duplicates())
            .join()
            .unwrap();
        assert_eq!(duplicates[0].table, "payment_intent");
        assert_eq!(duplicates[0].repeats, 1);
        assert_eq!(duplicates[0].first_read_from, expected_first);
        assert_eq!(duplicates[0].re_read_from, vec![expected_repeat]);
    }

    #[test]
    fn different_binds_are_different_records() {
        let ledger = ReadLedger::new();
        ledger.record_read("payment_intent", PI, None);
        assert!(!ledger.record_read("payment_intent", OTHER, None));
        assert!(ledger.take_duplicates().is_empty());
    }

    #[test]
    fn taking_clears_the_session() {
        let ledger = ReadLedger::new();
        ledger.record_read("payment_intent", PI, None);
        ledger.record_read("payment_intent", PI, None);
        assert_eq!(ledger.take_duplicates().len(), 1);
        // A second flush has nothing left to say.
        assert!(ledger.take_duplicates().is_empty());
    }

    #[test]
    fn an_untouched_ledger_has_nothing_to_report() {
        assert!(ReadLedger::new().take_duplicates().is_empty());
    }

    #[test]
    fn the_worst_record_is_reported_first() {
        let ledger = ReadLedger::new();
        ledger.record_read("payment_intent", PI, None);
        ledger.record_read("payment_intent", PI, None);
        ledger.record_read("merchant_account", OTHER, None);
        for _ in 0..4 {
            ledger.record_read("merchant_account", OTHER, None);
        }
        let duplicates = ledger.take_duplicates();
        assert_eq!(
            duplicates
                .iter()
                .map(|duplicate| (duplicate.table, duplicate.repeats))
                .collect::<Vec<_>>(),
            vec![("merchant_account", 4), ("payment_intent", 1)]
        );
    }

    #[test]
    fn repeats_are_counted_past_the_call_sites_named() {
        let ledger = ReadLedger::new();
        ledger.record_read("payment_intent", PI, None);
        for _ in 0..ORIGINS_PER_RECORD + 5 {
            ledger.record_read("payment_intent", PI, None);
        }
        let duplicate = ledger.take_duplicates().remove(0);
        assert_eq!(duplicate.repeats, ORIGINS_PER_RECORD + 5);
        assert_eq!(duplicate.re_read_from.len(), ORIGINS_PER_RECORD);
    }

    /// These two collided under a 64-bit `FxHash` fingerprint, which is what
    /// keying on a hash would have reported as one record read twice.
    #[test]
    fn statements_that_collide_when_hashed_stay_distinct() {
        let ledger = ReadLedger::new();
        assert!(!ledger.record_read("t", "SELECT 1908", None));
        assert!(!ledger.record_read("t", "SELECT 3600", None));
        assert!(ledger.take_duplicates().is_empty());
    }

    /// Every distinct statement is a different record, so none of them is
    /// dropped however many a session runs.
    #[test]
    fn distinct_statements_are_all_tracked() {
        let ledger = ReadLedger::new();
        for index in 0..5_000 {
            assert!(!ledger.record_read("t", &format!("SELECT {index}"), None));
        }
        assert!(ledger.record_read("t", "SELECT 0", None));
        assert!(ledger.record_read("t", "SELECT 4999", None));
        assert_eq!(ledger.take_duplicates().len(), 2);
    }
}
