//! The facade's hasher under a REPLAY hook.
//!
//! `set_global_runtime_hook` is a one-shot, so each runtime state has its own
//! test binary. This one installs a lookup-replay hook and drives
//! `HashMap::default()` inside an entered correlation, the path every map on a
//! request takes; the unit tests in `collections.rs` build the hasher directly
//! and never reach it.
#![cfg(feature = "deja")]
#![allow(clippy::panic, clippy::expect_used)]

use common_utils::collections::HashMap;

struct EmptyLookup;

impl deja::LookupTableSource for EmptyLookup {
    fn load(&mut self) -> std::io::Result<deja::LookupTable> {
        Ok(deja::LookupTable {
            recording_id: "empty".to_string(),
            policy_version: deja::POLICY_VERSION,
            entries: Vec::new(),
        })
    }
}

fn install_replay_hook() {
    let hook = deja::LookupTableHook::from_source(EmptyLookup, deja::InMemoryObservedSink::new())
        .expect("lookup replay hook");
    match deja::set_global_runtime_hook(Some(deja::RuntimeHook::LookupReplay(hook))) {
        Ok(()) => {}
        Err(_) if deja::runtime_mode().is_replay() => {}
        Err(err) => panic!("install replay hook: {err}"),
    }
}

/// Thirty-two keys, so two random key pairs render them alike with negligible
/// probability, and two derived pairs must render them identically.
fn rendered() -> String {
    let mut map: HashMap<u32, ()> = HashMap::default();
    for key in 0..32 {
        map.insert(key, ());
    }
    format!("{map:?}")
}

#[test]
fn inside_a_replayed_correlation_two_maps_iterate_alike() {
    install_replay_hook();
    let _correlation = deja::test_support::recording_correlation("corr-1");
    assert_eq!(rendered(), rendered());
}

#[test]
fn outside_a_correlation_the_keys_stay_random() {
    install_replay_hook();
    assert_ne!(rendered(), rendered());
}
