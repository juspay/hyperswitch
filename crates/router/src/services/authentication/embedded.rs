use std::time::Duration;

use common_utils::id_type;

use super::jwt;
use crate::{configs::Settings, consts, core::errors::UserResult};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EmbeddedToken {
    pub tenant_id: id_type::TenantId,
    pub org_id: id_type::OrganizationId,
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    exp: u64,
}

impl EmbeddedToken {
    pub async fn new(
        tenant_id: id_type::TenantId,
        org_id: id_type::OrganizationId,
        merchant_id: id_type::MerchantId,
        profile_id: id_type::ProfileId,
        settings: &Settings,
    ) -> UserResult<String> {
        let exp_duration = Duration::from_secs(consts::JWT_EMBEDDED_TOKEN_TIME_IN_SECS);
        let exp = jwt::generate_exp(exp_duration)?.as_secs();
        let token_payload = Self {
            tenant_id,
            org_id,
            merchant_id,
            profile_id,
            exp,
        };
        jwt::generate_jwt(&token_payload, settings).await
    }
}
