use common_utils::{id_type, pii};
use masking::Secret;

use crate::enums;

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct ReconUpdateMerchantRequest {
    pub recon_status: enums::ReconStatus,
    pub user_email: pii::Email,
}

#[derive(Debug, serde::Serialize)]
pub struct ReconTokenResponse {
    pub token: Secret<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct ReconStatusResponse {
    pub recon_status: enums::ReconStatus,
}

#[derive(serde::Serialize, Debug)]
pub struct VerifyTokenResponse {
    pub merchant_id: id_type::MerchantId,
    pub user_email: pii::Email,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acl: Option<String>,
}
