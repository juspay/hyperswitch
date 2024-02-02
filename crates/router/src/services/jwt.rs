use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use jsonwebtoken::{encode, EncodingKey, Header};
use masking::PeekInterface;

use super::authentication;
use crate::{configs::settings::Settings, core::errors::UserErrors};

/// This method generates an expiration time by adding the provided duration to the current system time. It then calculates the duration since the Unix epoch and returns it as a custom result. If any error occurs during the process, it changes the context of the error to InternalServerError and returns it as a custom result.
pub fn generate_exp(
    exp_duration: std::time::Duration,
) -> CustomResult<std::time::Duration, UserErrors> {
    std::time::SystemTime::now()
        .checked_add(exp_duration)
        .ok_or(UserErrors::InternalServerError)?
        .duration_since(std::time::UNIX_EPOCH)
        .into_report()
        .change_context(UserErrors::InternalServerError)
}

/// Asynchronously generates a JSON Web Token (JWT) using the provided claims data and settings.
///
/// # Arguments
///
/// * `claims_data` - The data to be serialized as the JWT claims.
/// * `settings` - The settings used to obtain the JWT secret.
///
/// # Returns
///
/// A `CustomResult` containing the generated JWT as a `String`, or a `UserErrors` if an error occurs.
///
pub async fn generate_jwt<T>(
    claims_data: &T,
    settings: &Settings,
) -> CustomResult<String, UserErrors>
where
    T: serde::ser::Serialize,
{
    let jwt_secret = authentication::get_jwt_secret(
        &settings.secrets,
        #[cfg(feature = "kms")]
        external_services::kms::get_kms_client(&settings.kms).await,
    )
    .await
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Failed to obtain JWT secret")?;
    encode(
        &Header::default(),
        claims_data,
        &EncodingKey::from_secret(jwt_secret.peek().as_bytes()),
    )
    .into_report()
    .change_context(UserErrors::InternalServerError)
}
