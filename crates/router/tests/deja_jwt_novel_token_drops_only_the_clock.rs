// Integration test: assertions use panic!/expect(); allow the production-code
// lints the v2 clippy profile denies.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]
//! A token absent from the recording is decoded with its signature verified and
//! its expiry ignored.
//!
//! The clock that decides a JWT's expiry is read by `jsonwebtoken` itself, from
//! `SystemTime::now()`, so the instrumented `date_time::now` boundary never
//! sees it and a replay cannot hold it still. That is the whole reason the
//! decode is a boundary, and it leaves the miss arm one answer that is neither
//! a constant nor a lie about the claims: drop the clock and keep everything
//! else. A fail-stop would blind the run from that point to the end of the
//! request, which is every comparison the replay existed to make.
//!
//! Dropping the clock does mean an expired token is accepted here. Dropping the
//! signature check would mean a forged one is, and that is not on the table:
//! the miss arm decodes with the real `DecodingKey::from_secret`, and
//! `validate_exp` is the only thing it turns off. `exp` also stays in
//! `required_spec_claims`, so "only the clock" is literal — a token that omits
//! `exp` is still `Invalid`, for a reason that has nothing to do with time.
//!
//! An empty lookup table makes every call a miss, so both halves are reachable
//! without a recorded entry, and the two tokens below differ only in the key
//! they were signed with. Own test binary: `set_global_runtime_hook` is a
//! one-shot `OnceLock`.
#![cfg(feature = "deja")]

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use router::services::authentication::{decode_jwt_verified, JwtDecodeOutcome};

/// The secret the caller holds. The forged token is signed with another one.
const SECRET: &[u8] = b"test-secret";

/// Fixed, because an expiry computed from the clock drifts back into validity
/// and stops testing anything.
const EXPIRED_AT: u64 = 1_600_000_000;

/// Only a decode of the token can produce this, so the claims assertion cannot
/// be satisfied by a synthesized or defaulted claim set.
const SUBJECT: &str = "deja-replay-expired-but-correctly-signed";

// `Debug` because `expect_err` requires it on the Ok type.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
}

#[allow(
    clippy::expect_used,
    reason = "test helper: a free fn, so allow-expect-in-tests does not cover it; a fixture that will not sign should fail the test loudly"
)]
fn signed_with(secret: &[u8]) -> String {
    encode(
        &Header::new(Algorithm::HS256),
        &Claims {
            sub: SUBJECT.to_string(),
            exp: EXPIRED_AT,
        },
        &EncodingKey::from_secret(secret),
    )
    .expect("sign the token")
}

#[test]
fn a_token_absent_from_the_recording_drops_only_the_clock() {
    let expired_but_signed = signed_with(SECRET);

    // A guard on the token, not a third claim about the seam: without it the
    // decode asserted below would also succeed on a token that is simply
    // valid, and would say nothing about the clock. This is the same
    // `jsonwebtoken` call the seam makes when deja is inactive, with
    // `validate_exp` left at its default.
    assert_eq!(
        decode::<Claims>(
            &expired_but_signed,
            &DecodingKey::from_secret(SECRET),
            &Validation::new(Algorithm::HS256),
        )
        .expect_err("the token must be expired, or this test proves nothing")
        .kind(),
        &jsonwebtoken::errors::ErrorKind::ExpiredSignature,
        "the token must be rejected for its expiry and nothing else"
    );

    let table = deja::LookupTable {
        recording_id: "jwt-novel-token-test".to_string(),
        policy_version: deja::POLICY_VERSION,
        event_schema_version: Some(deja::CURRENT_EVENT_SCHEMA_VERSION),
        entries: vec![],
        identity_entries: vec![],
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

    let claims = decode_jwt_verified::<Claims>(&expired_but_signed, SECRET)
        .expect("an expired but correctly signed token absent from the recording must decode");
    assert_eq!(
        claims.sub, SUBJECT,
        "the claims must come from the token and nowhere else"
    );
    assert_eq!(
        claims.exp, EXPIRED_AT,
        "the expiry the decode ignored must survive it unchanged"
    );

    // Header and payload bytes identical to the token above; the signing key is
    // the only difference, so this can be measuring one thing only.
    let forged = signed_with(b"not-the-secret-the-caller-holds");
    assert_eq!(
        decode_jwt_verified::<Claims>(&forged, SECRET)
            .expect_err("a token signed with the wrong secret must not decode")
            .current_context(),
        &JwtDecodeOutcome::Invalid,
        "ignoring the expiry must not become ignoring the signature"
    );
}
