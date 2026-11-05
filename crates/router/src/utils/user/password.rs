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
use redis_interface::{RedisKey, SetnxReply};
use router_env::logger;

use crate::{
    consts,
    core::errors::{UserErrors, UserResult},
    routes::SessionState,
};

pub fn generate_password_hash(
    password: Secret<String>,
) -> CustomResult<Secret<String>, UserErrors> {
    generate_password_hash_inner(password).map(Secret::new)
}

// deja: the Argon2 salt is random (OsRng), so the hash is non-deterministic. On
// replay the recorded user row never matches (the `users` INSERT then executes
// LIVE, collides with the record-phase user, and signup rolls everything back ->
// HE_00). Record/replay the hash string so the user row is reproducible. The
// annotated fn returns a PLAIN String (not Secret) on purpose: masking::Secret
// serializes lossily to "***", which would record/replay a useless masked value;
// the plain String records the real hash losslessly. The `password` arg still
// masks to "***" in the recorded args — that's fine, it's consistent across
// record/replay and avoids leaking the secret.
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "router::user::password",
        operation = "generate_password_hash",
        // Typed codec so a recorded `UserErrors` replays as the same error.
        codec = deja::codec::ResultCodec::<String, UserErrors>,
        // `is_correct_password` must parse a PHC string, so real Argon2 hashes
        // derived material; a later check then answers `Ok(false)`, not an error.
        on_miss = {
            use common_utils::synth_shape::Synthesize as _;
            let material = __deja_miss.alphanumeric(32);
            match SaltString::encode_b64(&deja::synth::bytes::<16>(&__deja_miss)).and_then(
                |salt| {
                    Argon2::default()
                        .hash_password(material.as_bytes(), &salt)
                        .map(|hash| hash.to_string())
                },
            ) {
                Ok(hash) => Ok(hash),
                // Unreachable for a well-formed 16-byte salt; keeps the arm total.
                Err(_) => Err(UserErrors::InternalServerError.into()),
            }
        },
    )
)]
fn generate_password_hash_inner(password: Secret<String>) -> CustomResult<String, UserErrors> {
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.expose().as_bytes(), &salt)
        .change_context(UserErrors::InternalServerError)?;
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

fn get_password_attempts_key(user_id: &str) -> String {
    format!(
        "{}{}",
        consts::user::REDIS_PASSWORD_ATTEMPTS_PREFIX,
        user_id
    )
}

fn get_password_lock_key(user_id: &str) -> String {
    format!("{}{}", consts::user::REDIS_PASSWORD_LOCK_PREFIX, user_id)
}

fn to_redis_seconds(seconds: u64) -> UserResult<i64> {
    i64::try_from(seconds).change_context(UserErrors::InternalServerError)
}

pub async fn is_password_sign_in_locked(state: &SessionState, user_id: &str) -> UserResult<bool> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    redis_conn
        .exists::<()>(&get_password_lock_key(user_id).into())
        .await
        .change_context(UserErrors::InternalServerError)
}

/// Records a password sign in attempt and returns the number of attempts made within the window,
/// including this one.
pub async fn record_password_attempt(state: &SessionState, user_id: &str) -> UserResult<usize> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    let key: RedisKey = get_password_attempts_key(user_id).into();
    let window_in_secs = to_redis_seconds(consts::user::PASSWORD_ATTEMPTS_WINDOW_IN_SECS)?;
    let now = common_utils::date_time::now_unix_timestamp();

    redis_conn
        .append_elements_to_list(&key, &[now.to_string()])
        .await
        .change_context(UserErrors::InternalServerError)?;
    redis_conn
        .set_expiry(&key, window_in_secs)
        .await
        .change_context(UserErrors::InternalServerError)?;

    // One more attempt than are kept is read, which shows whether the list needs trimming
    let read_from = i64::try_from(consts::user::PASSWORD_ATTEMPTS_TO_KEEP + 1)
        .map(|count| -count)
        .change_context(UserErrors::InternalServerError)?;
    let attempts = redis_conn
        .get_list_elements(&key, read_from, -1)
        .await
        .change_context(UserErrors::InternalServerError)?;

    if attempts.len() > consts::user::PASSWORD_ATTEMPTS_TO_KEEP {
        let _ = redis_conn
            .lpop_list_elements(&key, Some(1))
            .await
            .inspect_err(|error| logger::error!(?error));
    }

    let window_start = now.saturating_sub(window_in_secs);
    Ok(attempts
        .iter()
        .rev()
        .take(consts::user::PASSWORD_ATTEMPTS_TO_KEEP)
        .filter_map(|attempt| attempt.parse::<i64>().ok())
        .filter(|attempted_at| *attempted_at > window_start)
        .count())
}

async fn set_password_lock(state: &SessionState, user_id: &str) -> UserResult<SetnxReply> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    redis_conn
        .set_key_if_not_exists_with_expiry(
            &get_password_lock_key(user_id).into(),
            common_utils::date_time::now_unix_timestamp(),
            Some(to_redis_seconds(
                consts::user::PASSWORD_LOCK_DURATION_IN_SECS,
            )?),
        )
        .await
        .change_context(UserErrors::InternalServerError)
}

/// Locks password sign in for the lock duration, unless it is locked already. A failure to lock is
/// only logged, as the attempts stay recorded and the next attempt locks it again.
pub async fn lock_password_sign_in(state: &SessionState, user_id: &str) {
    match set_password_lock(state, user_id).await {
        Ok(SetnxReply::KeySet) => {
            logger::warn!(
                user_id,
                "Password sign in locked after too many failed attempts"
            );
        }
        Ok(SetnxReply::KeyNotSet) => {}
        Err(error) => logger::error!(?error, "Failed to lock password sign in"),
    }
}

pub async fn clear_password_attempts(state: &SessionState, user_id: &str) -> UserResult<()> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    redis_conn
        .delete_key(&get_password_attempts_key(user_id).into())
        .await
        .change_context(UserErrors::InternalServerError)
        .map(|_| ())
}

#[cfg(all(test, feature = "deja"))]
mod deja_tests {
    use super::UserErrors;

    type Seam = deja::codec::ResultCodec<String, UserErrors>;

    fn capture(
        value: &common_utils::errors::CustomResult<String, UserErrors>,
    ) -> (serde_json::Value, bool) {
        <Seam as deja::codec::ReplayCodec>::capture(value)
    }

    fn reconstruct(
        recorded: serde_json::Value,
    ) -> Option<common_utils::errors::CustomResult<String, UserErrors>> {
        <Seam as deja::codec::ReplayCodec>::reconstruct(recorded)
    }

    /// A captured `Err` records its variant as `kind` and reconstructs as it.
    #[test]
    fn a_captured_error_round_trips_as_its_variant() {
        let (recorded, is_error) = capture(&Err(UserErrors::InternalServerError.into()));
        assert!(is_error, "an Err must be captured as an error");
        assert_eq!(
            recorded.get("kind").and_then(serde_json::Value::as_str),
            Some("InternalServerError"),
            "the recorded kind must name the variant: {recorded}"
        );
        let Some(Err(report)) = reconstruct(recorded) else {
            panic!("a captured error must reconstruct as an error");
        };
        assert!(matches!(
            report.current_context(),
            UserErrors::InternalServerError
        ));
    }

    /// A recorded error rebuilds as its variant; a sentinel or unknown `kind` refuses.
    #[test]
    fn a_recorded_error_rebuilds_its_variant() {
        let rebuilt = reconstruct(serde_json::json!({
            "version": 1,
            "result": "Err",
            "kind": "InternalServerError",
            "message": "User InternalServerError",
        }))
        .expect("a typed error must reconstruct");
        let Err(report) = &rebuilt else {
            panic!("a recorded error must rebuild as an error");
        };
        assert!(
            matches!(report.current_context(), UserErrors::InternalServerError),
            "the rebuilt error must carry the recorded variant"
        );

        assert!(
            reconstruct(serde_json::json!({"deja_err": "InternalServerError"})).is_none(),
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

    /// The seam's attribute selects the typed codec; read from source, since the
    /// macro expansion is not observable at run time.
    #[test]
    fn the_seam_selects_the_typed_result_codec() {
        let source = include_str!("password.rs");
        let (_, after_operation) = source
            .split_once("operation = \"generate_password_hash\",")
            .expect("the seam must declare its operation");
        let (declaration, _) = after_operation
            .split_once(")]")
            .expect("the seam's attribute must be closed");
        assert!(
            declaration.contains("codec = deja::codec::ResultCodec::<String, UserErrors>"),
            "the password-hash seam must select the typed result codec, or a \
             recorded failure replays as an unreconstructable sentinel: {declaration}"
        );
    }
}
