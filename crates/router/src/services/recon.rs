use common_utils::{ext_traits::OptionExt, id_type};
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
    pub merchant_id: id_type::MerchantId,
    pub org_id: id_type::OrganizationId,
    pub profile_id: Option<id_type::ProfileId>,
}

impl ReconToken {
    pub async fn new_token(
        settings: &Settings,
        user_role: &diesel_models::user_role::UserRole,
    ) -> RouterResult<Secret<String>> {
        let exp_duration = std::time::Duration::from_secs(consts::JWT_TOKEN_TIME_IN_SECS);
        let exp = jwt::generate_exp(exp_duration)
            .change_context(core::errors::ApiErrorResponse::InternalServerError)?
            .as_secs();
        let merchant_id = user_role
            .merchant_id
            .clone()
            .get_required_value("merchant_id in user_role")
            .change_context(core::errors::ApiErrorResponse::MissingRequiredField {
                field_name: "merchant_id",
            })?;
        let org_id = user_role
            .org_id
            .clone()
            .get_required_value("org_id in user_role")
            .change_context(core::errors::ApiErrorResponse::MissingRequiredField {
                field_name: "org_id",
            })?;
        let token_payload = Self {
            user_id: user_role.user_id.clone(),
            exp,
            merchant_id,
            org_id,
            profile_id: user_role.profile_id.clone(),
        };
        let token = jwt::generate_jwt(&token_payload, settings)
            .await
            .change_context(core::errors::ApiErrorResponse::InternalServerError)?;
        Ok(Secret::new(token))
    }
}
