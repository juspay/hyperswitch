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
