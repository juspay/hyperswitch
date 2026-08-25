use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_masking::PeekInterface;
use jsonwebtoken::{encode, EncodingKey, Header};

use crate::{configs::Settings, core::errors::UserErrors};

// deja: JWT expiry uses a raw SystemTime::now() that bypasses the instrumented
// date_time::now boundary, so the `exp` claim (and thus the whole signed token)
// diverges on replay. Record/replay the absolute expiry (Ok-only) to reproduce
// byte-identical JWTs.
#[cfg_attr(
    feature = "deja",
    deja::id(component = "router::jwt", operation = "generate_exp", codec = ResultOkCodec,)
)]
pub fn generate_exp(
    exp_duration: std::time::Duration,
) -> CustomResult<std::time::Duration, UserErrors> {
    std::time::SystemTime::now()
        .checked_add(exp_duration)
        .ok_or(UserErrors::InternalServerError)?
        .duration_since(std::time::UNIX_EPOCH)
        .change_context(UserErrors::InternalServerError)
}

pub async fn generate_jwt<T>(
    claims_data: &T,
    settings: &Settings,
) -> CustomResult<String, UserErrors>
where
    T: serde::ser::Serialize,
{
    let jwt_secret = &settings.secrets.get_inner().jwt_secret;
    encode(
        &Header::default(),
        claims_data,
        &EncodingKey::from_secret(jwt_secret.peek().as_bytes()),
    )
    .change_context(UserErrors::InternalServerError)
}
