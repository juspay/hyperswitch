use error_stack::ResultExt;
use masking::Secret;

use super::jwt;
use crate::{
    consts,
    core::{self, errors::RouterResult},
    routes::app::settings::Settings,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ReconToken {
    pub user_id: String,
    pub exp: u64,
}

impl ReconToken {
        /// Creates a new JWT token for the specified user ID using the provided settings.
    /// 
    /// # Arguments
    /// 
    /// * `user_id` - A string representing the user ID for which the token is being generated.
    /// * `settings` - A reference to the Settings struct containing the JWT configuration.
    /// 
    /// # Returns
    /// 
    /// A `RouterResult` containing the newly generated JWT token as a `Secret<String>`, or an error if the token generation fails.
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
