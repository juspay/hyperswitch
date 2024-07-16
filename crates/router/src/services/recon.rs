use error_stack::ResultExt;
use masking::Secret;

use super::jwt;
use crate::{
    configs::Settings,
    consts,
    core::{self, errors::RouterResult},
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ReconToken {
    pub user_id: String,
    pub exp: u64,
}

impl ReconToken {
    pub async fn new_token(user_id: String, settings: &Settings) -> RouterResult<Secret<String>> {
        let exp_duration = std::time::Duration::from_secs(consts::JWT_TOKEN_TIME_IN_SECS);
        let exp = jwt::generate_exp(exp_duration)
            .change_context(core::errors::ApiErrorResponse::InternalServerError)?
            .as_secs();
        let token_payload = Self { user_id, exp };
        let token = jwt::generate_jwt(&token_payload, settings)
            .await
            .change_context(core::errors::ApiErrorResponse::InternalServerError)?;
        Ok(Secret::new(token))
    }
}
