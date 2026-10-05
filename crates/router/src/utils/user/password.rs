use argon2::{
    password_hash::{
        rand_core::OsRng, Error as argon2Err, PasswordHash, PasswordHasher, PasswordVerifier,
        SaltString,
    },
    Argon2,
};
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};
use rand::{seq::SliceRandom, Rng};

use crate::core::errors::UserErrors;

pub fn generate_password_hash(
    password: Secret<String>,
) -> CustomResult<Secret<String>, UserErrors> {
    generate_password_hash_inner(password)
        .map(Secret::new)
        .change_context(UserErrors::InternalServerError)
}

/// The decision Argon2 reaches about a password, as something that can be
/// recorded.
///
/// [`UserErrors`] cannot be: it derives neither `Serialize` nor `Deserialize`,
/// and it carries payload-bearing variants that have nothing to do with this
/// call. Every `hash_password` failure is reported as
/// `UserErrors::InternalServerError`, which is all a caller can tell apart, so
/// that is this one variant and nothing else crosses the boundary. The mapping
/// back to the public error type stays in [`generate_password_hash`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, thiserror::Error,
)]
enum PasswordHashOutcome {
    /// Argon2 refused to hash the password.
    #[error("failed to hash the password")]
    HashFailed,
}

// deja: the Argon2 salt is random (OsRng), so the hash is non-deterministic. On
// replay the recorded user row never matches (the `users` INSERT then executes
// LIVE, collides with the record-phase user, and signup rolls everything back ->
// HE_00). Record/replay the hash string so the user row is reproducible. The
// annotated fn returns a PLAIN String (not Secret) on purpose: masking::Secret
// serializes lossily to "***", which would record/replay a useless masked value;
// the plain String records the real hash losslessly. The `password` arg still
// masks to "***" in the recorded args — that's fine, it's consistent across
// record/replay and avoids leaking the secret. The error side is its own narrow
// outcome type so a recorded failure replays as that failure rather than as an
// unreconstructable sentinel.
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "router::user::password",
        operation = "generate_password_hash",
        codec = deja::codec::ResultCodec::<String, PasswordHashOutcome>,
    )
)]
fn generate_password_hash_inner(
    password: Secret<String>,
) -> CustomResult<String, PasswordHashOutcome> {
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.expose().as_bytes(), &salt)
        .change_context(PasswordHashOutcome::HashFailed)?;
    Ok(password_hash.to_string())
}

pub fn is_correct_password(
    candidate: &Secret<String>,
    password: &Secret<String>,
) -> CustomResult<bool, UserErrors> {
    let password = password.peek();
    let parsed_hash =
        PasswordHash::new(password).change_context(UserErrors::InternalServerError)?;
    let result = Argon2::default().verify_password(candidate.peek().as_bytes(), &parsed_hash);
    match result {
        Ok(_) => Ok(true),
        Err(argon2Err::Password) => Ok(false),
        Err(e) => Err(e),
    }
    .change_context(UserErrors::InternalServerError)
}

pub fn get_index_for_correct_recovery_code(
    candidate: &Secret<String>,
    recovery_codes: &[Secret<String>],
) -> CustomResult<Option<usize>, UserErrors> {
    for (index, recovery_code) in recovery_codes.iter().enumerate() {
        let is_match = is_correct_password(candidate, recovery_code)?;
        if is_match {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

pub fn get_temp_password() -> Secret<String> {
    Secret::new(get_temp_password_inner())
}

// deja: the temporary password is emailed to the user, so it reaches an outbound
// request and must be reproducible on replay. Returns a plain `String` because
// masking::Secret serializes lossily to "***" -- see
// `generate_password_hash_inner`. The caller re-wraps immediately.
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "router::user::password",
        operation = "get_temp_password",
        on_miss = { use common_utils::synth_shape::Synthesize as _; __deja_miss.uuid().to_string() },
        codec = SerdeCodec,
    )
)]
fn get_temp_password_inner() -> String {
    let uuid_pass = common_utils::generate_uuid_v4().to_string();

    #[allow(
        clippy::disallowed_methods,
        reason = "this function IS the seam: the whole password is recorded as one value"
    )]
    let mut rng = rand::thread_rng();

    let special_chars: Vec<char> = "!@#$%^&*()-_=+[]{}|;:,.<>?".chars().collect();
    let special_char = special_chars.choose(&mut rng).unwrap_or(&'@');

    format!(
        "{}{}{}{}{}",
        uuid_pass,
        rng.gen_range('A'..='Z'),
        special_char,
        rng.gen_range('a'..='z'),
        rng.gen_range('0'..='9'),
    )
}

#[cfg(all(test, feature = "deja"))]
mod deja_tests {
    use super::PasswordHashOutcome;

    fn reconstruct(
        recorded: serde_json::Value,
    ) -> Option<common_utils::errors::CustomResult<String, PasswordHashOutcome>> {
        <deja::codec::ResultCodec<String, PasswordHashOutcome> as deja::codec::ReplayCodec>::reconstruct(recorded)
    }

    /// A recorded hashing failure rebuilds as the same variant.
    ///
    /// The Ok-only codec this site used wrote an `Err` as
    /// `{"deja_err": "<Debug>"}`, which names no variant and so cannot rebuild
    /// one -- the second case is that document, and it must still refuse. The
    /// third case is what makes the first mean anything: a single-variant enum
    /// whose `kind` went unread would accept any string at all.
    #[test]
    fn a_recorded_error_rebuilds_its_variant() {
        let rebuilt = reconstruct(serde_json::json!({
            "version": 1,
            "result": "Err",
            "kind": "HashFailed",
            "message": "failed to hash the password",
        }))
        .expect("a typed error must reconstruct");
        let Err(report) = &rebuilt else {
            panic!("a recorded error must rebuild as an error");
        };
        assert_eq!(report.current_context(), &PasswordHashOutcome::HashFailed);

        assert!(
            reconstruct(serde_json::json!({"deja_err": "HashFailed"})).is_none(),
            "the Ok-only sentinel names no variant and must refuse"
        );
        assert!(
            reconstruct(serde_json::json!({
                "version": 1,
                "result": "Err",
                "kind": "NotAVariant",
                "message": "",
            }))
            .is_none(),
            "a kind naming no variant must refuse rather than fabricate one"
        );
    }
}
