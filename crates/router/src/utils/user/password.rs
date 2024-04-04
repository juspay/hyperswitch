use argon2::{
    password_hash::{
        rand_core::OsRng, Error as argon2Err, PasswordHash, PasswordHasher, PasswordVerifier,
        SaltString,
    },
    Argon2,
};
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};

use crate::core::errors::UserErrors;

pub fn generate_password_hash(
    password: Secret<String>,
) -> CustomResult<Secret<String>, UserErrors> {
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.expose().as_bytes(), &salt)
        .change_context(UserErrors::InternalServerError)?;
    Ok(Secret::new(password_hash.to_string()))
}

pub fn is_correct_password(
    candidate: Secret<String>,
    password: Secret<String>,
) -> CustomResult<bool, UserErrors> {
    let password = password.expose();
    let parsed_hash =
        PasswordHash::new(&password).change_context(UserErrors::InternalServerError)?;
    let result = Argon2::default().verify_password(candidate.expose().as_bytes(), &parsed_hash);
    match result {
        Ok(_) => Ok(true),
        Err(argon2Err::Password) => Ok(false),
        Err(e) => Err(e),
    }
    .change_context(UserErrors::InternalServerError)
}
