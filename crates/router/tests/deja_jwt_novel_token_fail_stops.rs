// Integration test: assertions use panic!/expect(); allow the production-code
// lints the v2 clippy profile denies.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]
//! A token the recording has never seen must not decode silently on replay.
//!
//! Substituting the decode outcome is what takes the clock out of the replay
//! path, but it must not become a way for an unknown credential to be accepted.
//! An empty lookup table guarantees a miss for any call, so this is decisive
//! without constructing a matching key: the boundary fail-stops, and the real
//! `jsonwebtoken` decode never runs.
//!
//! Own test binary: `set_global_runtime_hook` is a one-shot `OnceLock`.
#![cfg(feature = "deja")]

use router::services::authentication::decode_jwt_verified;

// `Debug` because `expect_err` requires it on the Ok type.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Claims {
    sub: String,
}

#[test]
fn a_token_absent_from_the_recording_fail_stops() {
    let table = deja::LookupTable {
        recording_id: "jwt-novel-token-test".to_string(),
        policy_version: 1,
        entries: vec![],
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("lookup.json");
    std::fs::write(&path, serde_json::to_vec(&table).expect("serialize")).expect("write table");

    let hook = deja::LookupTableHook::from_source(
        deja::LocalFileLookupSource::new(path),
        deja::InMemoryObservedSink::new(),
    )
    .expect("hook");
    deja::set_global_runtime_hook(Some(deja::RuntimeHook::LookupReplay(hook)))
        .expect("install replay hook");

    // The fail-stop's panic backtrace is expected; silence the default hook.
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(|| {
        decode_jwt_verified::<Claims>("a-token-this-recording-has-never-seen", b"test-secret")
    });
    std::panic::set_hook(prev);

    let payload = result.expect_err(
        "a token absent from the recording must FAIL-STOP — not fall through to a real decode",
    );
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("<non-string panic payload>");
    assert!(
        message.contains("deja replay fail-stop") && message.contains("decode_jwt"),
        "the fail-stop must name the boundary that missed; got: {message:?}"
    );
}
