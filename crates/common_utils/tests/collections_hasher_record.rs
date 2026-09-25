//! The facade's hasher under a RECORD hook, per sampling decision.
//!
//! One binary per runtime state, since the hook is a one-shot. The decisions
//! are set on the registry the way `router_env::request_id` sets them, before
//! the correlation is entered.
#![cfg(feature = "deja")]
#![allow(clippy::panic, clippy::expect_used)]

use common_utils::collections::HashMap;

#[derive(Clone)]
struct NullSink;

impl deja::RecordSink<deja::DejaRecord> for NullSink {
    fn write_batch(&mut self, _records: &[deja::DejaRecord]) -> std::io::Result<()> {
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn install_record_hook() {
    let hook = deja::RecordingHook::with_sink(
        NullSink,
        "collections-hasher-record".to_string(),
        deja::WriterConfig::default(),
    );
    match deja::set_global_runtime_hook(Some(deja::RuntimeHook::Recording(std::sync::Arc::new(
        hook,
    )))) {
        Ok(()) => {}
        Err(_) if !deja::runtime_mode_is_disabled() => {}
        Err(err) => panic!("install recording hook: {err}"),
    }
}

fn rendered() -> String {
    let mut map: HashMap<u32, ()> = HashMap::default();
    for key in 0..32 {
        map.insert(key, ());
    }
    format!("{map:?}")
}

#[test]
fn a_recorded_request_derives_its_keys() {
    install_record_hook();
    deja::set_recording_decision("recorded", true);
    let _correlation = deja::test_support::recording_correlation("recorded");
    assert_eq!(rendered(), rendered());
}

#[test]
fn a_skipped_request_keeps_random_keys() {
    install_record_hook();
    deja::set_recording_decision("skipped", false);
    let _correlation = deja::test_support::recording_correlation("skipped");
    assert_ne!(rendered(), rendered());
}

/// A request whose decision was never set derives. That is what makes the
/// producer's unconditional push load-bearing: see the request_id test.
#[test]
fn a_request_with_no_decision_derives_its_keys() {
    install_record_hook();
    let _correlation = deja::test_support::recording_correlation("undecided");
    assert_eq!(deja::recording_decision("undecided"), None);
    assert_eq!(rendered(), rendered());
}
