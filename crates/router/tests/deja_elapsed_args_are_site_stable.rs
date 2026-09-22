// Integration test: assertions use panic!/expect(); allow the production-code
// lints the v2 clippy profile denies.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]
//! The elapsed seam must address itself by its site, not by the clock it reads.
//!
//! Its parameter is an `Instant`, which is not serialisable and so is captured
//! through `Debug` — a pair of process-relative counters that differs on every
//! call. Left in the arguments it re-keys the site each time, the lookup never
//! hits, the miss arm answers every call, and the reading lands in the outgoing
//! request. That is the class the seam exists to remove.
//!
//! Own test binary: `set_global_runtime_hook` is a one-shot `OnceLock`.
#![cfg(feature = "deja")]

#[test]
fn two_readings_from_different_instants_share_one_address() {
    let table = deja::LookupTable {
        recording_id: "elapsed-args-test".to_string(),
        policy_version: deja::POLICY_VERSION,
        entries: vec![],
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("lookup.json");
    std::fs::write(&path, serde_json::to_vec(&table).expect("serialize")).expect("write table");

    let sink = deja::InMemoryObservedSink::new();
    let observed = sink.handle();
    let hook = deja::LookupTableHook::from_source(deja::LocalFileLookupSource::new(path), sink)
        .expect("hook");
    deja::set_global_runtime_hook(Some(deja::RuntimeHook::LookupReplay(hook)))
        .expect("install replay hook");

    let first_start = std::time::Instant::now();
    // Spin until the clock has moved, so the two origins cannot coincide.
    while first_start.elapsed().as_nanos() == 0 {}
    let second_start = std::time::Instant::now();
    assert_ne!(
        format!("{first_start:?}"),
        format!("{second_start:?}"),
        "the two origins must differ, or this proves nothing"
    );

    let _ = common_utils::elapsed::millis_since(first_start);
    let _ = common_utils::elapsed::millis_since(second_start);

    let calls = observed.lock().expect("observed calls");
    let readings: Vec<&deja::ObservedCall> = calls
        .iter()
        .filter(|call| call.method_name == "millis_since")
        .collect();
    // Destructured rather than indexed: the count is checked by the pattern, so
    // a run that observed the wrong number of calls says so instead of the
    // assertions below reading a slot that is not there.
    let [first, second] = readings.as_slice() else {
        panic!(
            "both readings must be observed, or the assertions below are vacuous; got {}",
            readings.len()
        );
    };
    assert_eq!(
        first.args, second.args,
        "two readings taken from different origins must address the same site"
    );
    assert_eq!(
        first.args,
        serde_json::json!({}),
        "and the site carries no arguments at all: its identity is positional"
    );
}
