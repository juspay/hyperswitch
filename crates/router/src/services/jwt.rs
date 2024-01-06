use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use jsonwebtoken::{encode, EncodingKey, Header};
use masking::PeekInterface;

use super::authentication;
use crate::{configs::settings::Settings, core::errors::UserErrors};

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

pub async fn generate_jwt<T>(
    claims_data: &T,
    settings: &Settings,
) -> CustomResult<String, UserErrors>
where
    T: serde::ser::Serialize,
{
    let jwt_secret = authentication::get_jwt_secret(
        &settings.secrets,
        #[cfg(feature = "aws_kms")]
        external_services::aws_kms::get_aws_kms_client(&settings.kms).await,
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
