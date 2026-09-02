// Integration test: assertions use panic!/expect(); allow the production-code
// lints the v2 clippy profile denies.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]
//! A recording must be able to pair a replayed `decode_jwt` with its recorded
//! outcome without carrying the token that produced it.
//!
//! A tape lives in object storage for weeks and is read by anyone who can read
//! the bucket. A dashboard JWT stays valid for two days, so a tape that quoted
//! one would be handing out a live credential. The boundary therefore records a
//! digest, and this test reads the artifact bytes back off disk and asserts the
//! token itself is nowhere in them.
//!
//! Own test binary: `set_global_runtime_hook` is a one-shot `OnceLock`, so the
//! recording install here cannot share a process with the replay install in
//! `deja_jwt_novel_token_fail_stops.rs`.
#![cfg(feature = "deja")]

use router::services::authentication::{decode_jwt_verified, JwtDecodeOutcome};

/// Not a real JWT. The boundary records the outcome whichever way the decode
/// goes, and `Err(Invalid)` exercises the same capture path as a success while
/// keeping the test free of a signing fixture.
const TOKEN: &str = "deja-tape-probe.this-string-must-not-reach-the-tape.0123456789";

const CORRELATION_ID: &str = "req-deja-jwt-tape-excludes-token";

// `Debug` because `expect_err` requires it on the Ok type.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Claims {
    sub: String,
}

fn artifact_bytes(dir: &std::path::Path) -> Vec<u8> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(&path).expect("read artifact dir") {
            let entry = entry.expect("dir entry");
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
            } else {
                out.extend(std::fs::read(&entry_path).expect("read artifact file"));
            }
        }
    }
    out
}

#[test]
fn recorded_call_carries_a_digest_and_never_the_token() {
    let artifacts = tempfile::tempdir().expect("tempdir");
    deja::set_global_runtime_hook(Some(deja::RuntimeHook::Recording(std::sync::Arc::new(
        deja::RecordingHook::new(artifacts.path()).expect("recording hook"),
    ))))
    .expect("install recording hook");
    // Recording is gated on a correlation being CURRENT, not merely on a
    // decision being registered for one — an earlier version of this test set
    // the decision alone and recorded nothing at all. The guard must outlive
    // the call below, so it is bound rather than dropped.
    let _correlation = deja::test_support::recording_correlation(CORRELATION_ID);

    let outcome = decode_jwt_verified::<Claims>(TOKEN, b"test-secret");
    assert_eq!(
        outcome
            .expect_err("a non-JWT string cannot decode")
            .current_context(),
        &JwtDecodeOutcome::Invalid,
        "a malformed token is Invalid, not Expired"
    );

    deja::flush_global_runtime_hook().ok();
    let bytes = artifact_bytes(artifacts.path());
    assert!(
        !bytes.is_empty(),
        "the boundary must have recorded something to assert about"
    );

    let haystack = String::from_utf8_lossy(&bytes);
    assert!(
        !haystack.contains(TOKEN),
        "the raw token must never appear in a recording artifact"
    );
    let digest = blake3::hash(TOKEN.as_bytes()).to_hex().to_string();
    assert!(
        haystack.contains(&digest),
        "the recording must carry the token's digest, or replay cannot pair the call"
    );
}
