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
// record/replay and avoids leaking the secret. Uses the Ok-only codec for the Result.
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "router::user::password",
        operation = "generate_password_hash",
        codec = ResultOkCodec,
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
    let uuid_pass = uuid::Uuid::new_v4().to_string();
    let mut rng = rand::thread_rng();

    let special_chars: Vec<char> = "!@#$%^&*()-_=+[]{}|;:,.<>?".chars().collect();
    let special_char = special_chars.choose(&mut rng).unwrap_or(&'@');

    Secret::new(format!(
        "{}{}{}{}{}",
        uuid_pass,
        rng.gen_range('A'..='Z'),
        special_char,
        rng.gen_range('a'..='z'),
        rng.gen_range('0'..='9'),
    ))
}
