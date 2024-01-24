use common_utils::pii;
use masking::Secret;

use crate::enums;

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct ReconUpdateMerchantRequest {
    pub merchant_id: String,
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
